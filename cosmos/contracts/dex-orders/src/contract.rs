use cosmwasm_std::{
    entry_point, to_json_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Order as StdOrder,
    Response, StdResult, Uint128,
};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{
    BestPricesResponse, ExecuteMsg, InstantiateMsg, OrderResponse, OrderbookLevel,
    OrderbookResponse, OrdersResponse, QueryMsg,
};
use crate::state::{
    price_key, Order, OrderSide, OrderStatus, BUY_ORDERS, CREATOR_ORDERS, NEXT_ORDER_ID, ORDERS,
    SELL_ORDERS,
};

const CONTRACT_NAME: &str = "crates.io:dex-orders";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    NEXT_ORDER_ID.save(deps.storage, &1u64)?;

    Ok(Response::new().add_attribute("method", "instantiate"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::CreateBuyOrder {
            base_token,
            quote_token,
            base_amount,
            price,
            expiry,
        } => execute_create_buy_order(
            deps,
            env,
            info,
            base_token,
            quote_token,
            base_amount,
            price,
            expiry,
        ),
        ExecuteMsg::CreateSellOrder {
            base_token,
            quote_token,
            base_amount,
            price,
            expiry,
        } => execute_create_sell_order(
            deps,
            env,
            info,
            base_token,
            quote_token,
            base_amount,
            price,
            expiry,
        ),
        ExecuteMsg::CancelOrder { order_id } => execute_cancel_order(deps, info, order_id),
        ExecuteMsg::FillOrder { order_id, amount } => {
            execute_fill_order(deps, env, info, order_id, amount)
        }
    }
}

pub fn execute_create_buy_order(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    base_token: String,
    quote_token: String,
    base_amount: Uint128,
    price: Uint128,
    expiry: Option<u64>,
) -> Result<Response, ContractError> {
    if base_amount.is_zero() {
        return Err(ContractError::InvalidAmount {});
    }
    if price.is_zero() {
        return Err(ContractError::InvalidPrice {});
    }

    let order_id = NEXT_ORDER_ID.load(deps.storage)?;
    NEXT_ORDER_ID.save(deps.storage, &(order_id + 1))?;

    let order = Order {
        id: order_id,
        creator: info.sender.clone(),
        base_token: base_token.clone(),
        quote_token: quote_token.clone(),
        side: OrderSide::Buy,
        base_amount,
        filled_amount: Uint128::zero(),
        price,
        expiry: expiry.unwrap_or(0),
        created_at: env.block.time.seconds(),
        status: OrderStatus::Active,
    };

    ORDERS.save(deps.storage, order_id, &order)?;
    CREATOR_ORDERS.save(deps.storage, (&info.sender, order_id), &())?;

    // Add to buy orders index
    let key = (base_token.clone(), quote_token.clone(), price_key(price));
    let mut order_ids = BUY_ORDERS.may_load(deps.storage, key.clone())?.unwrap_or_default();
    order_ids.push(order_id);
    BUY_ORDERS.save(deps.storage, key, &order_ids)?;

    // Try to match with existing sell orders
    try_match_order(deps, env, order_id)?;

    Ok(Response::new()
        .add_attribute("method", "create_buy_order")
        .add_attribute("order_id", order_id.to_string())
        .add_attribute("base_token", base_token)
        .add_attribute("quote_token", quote_token)
        .add_attribute("price", price)
        .add_attribute("amount", base_amount))
}

pub fn execute_create_sell_order(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    base_token: String,
    quote_token: String,
    base_amount: Uint128,
    price: Uint128,
    expiry: Option<u64>,
) -> Result<Response, ContractError> {
    if base_amount.is_zero() {
        return Err(ContractError::InvalidAmount {});
    }
    if price.is_zero() {
        return Err(ContractError::InvalidPrice {});
    }

    let order_id = NEXT_ORDER_ID.load(deps.storage)?;
    NEXT_ORDER_ID.save(deps.storage, &(order_id + 1))?;

    let order = Order {
        id: order_id,
        creator: info.sender.clone(),
        base_token: base_token.clone(),
        quote_token: quote_token.clone(),
        side: OrderSide::Sell,
        base_amount,
        filled_amount: Uint128::zero(),
        price,
        expiry: expiry.unwrap_or(0),
        created_at: env.block.time.seconds(),
        status: OrderStatus::Active,
    };

    ORDERS.save(deps.storage, order_id, &order)?;
    CREATOR_ORDERS.save(deps.storage, (&info.sender, order_id), &())?;

    // Add to sell orders index
    let key = (base_token.clone(), quote_token.clone(), price_key(price));
    let mut order_ids = SELL_ORDERS.may_load(deps.storage, key.clone())?.unwrap_or_default();
    order_ids.push(order_id);
    SELL_ORDERS.save(deps.storage, key, &order_ids)?;

    // Try to match with existing buy orders
    try_match_order(deps, env, order_id)?;

    Ok(Response::new()
        .add_attribute("method", "create_sell_order")
        .add_attribute("order_id", order_id.to_string())
        .add_attribute("base_token", base_token)
        .add_attribute("quote_token", quote_token)
        .add_attribute("price", price)
        .add_attribute("amount", base_amount))
}

fn try_match_order(
    _deps: DepsMut,
    _env: Env,
    _order_id: u64,
) -> Result<(), ContractError> {
    // Simplified matching - production would implement full matching engine
    // Would iterate through opposite side's orderbook and execute fills
    Ok(())
}

pub fn execute_cancel_order(
    deps: DepsMut,
    info: MessageInfo,
    order_id: u64,
) -> Result<Response, ContractError> {
    ORDERS.update(deps.storage, order_id, |maybe_order| {
        let mut order = maybe_order.ok_or(ContractError::OrderNotFound {})?;

        if info.sender != order.creator {
            return Err(ContractError::Unauthorized {});
        }

        if !matches!(
            order.status,
            OrderStatus::Active | OrderStatus::PartiallyFilled
        ) {
            return Err(ContractError::OrderNotActive {});
        }

        order.status = OrderStatus::Cancelled;
        Ok(order)
    })?;

    Ok(Response::new()
        .add_attribute("method", "cancel_order")
        .add_attribute("order_id", order_id.to_string()))
}

pub fn execute_fill_order(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    order_id: u64,
    amount: Uint128,
) -> Result<Response, ContractError> {
    if amount.is_zero() {
        return Err(ContractError::InvalidAmount {});
    }

    ORDERS.update(deps.storage, order_id, |maybe_order| {
        let mut order = maybe_order.ok_or(ContractError::OrderNotFound {})?;

        if info.sender == order.creator {
            return Err(ContractError::SelfMatch {});
        }

        if !matches!(
            order.status,
            OrderStatus::Active | OrderStatus::PartiallyFilled
        ) {
            return Err(ContractError::OrderNotActive {});
        }

        if order.expiry > 0 && env.block.time.seconds() >= order.expiry {
            return Err(ContractError::OrderExpired {});
        }

        let remaining = order.base_amount.checked_sub(order.filled_amount)?;
        let fill_amount = amount.min(remaining);

        order.filled_amount = order.filled_amount.checked_add(fill_amount)?;

        if order.filled_amount >= order.base_amount {
            order.status = OrderStatus::Filled;
        } else {
            order.status = OrderStatus::PartiallyFilled;
        }

        // In production: Execute token transfers based on order side
        Ok(order)
    })?;

    Ok(Response::new()
        .add_attribute("method", "fill_order")
        .add_attribute("order_id", order_id.to_string())
        .add_attribute("amount", amount))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetOrder { order_id } => to_json_binary(&query_order(deps, order_id)?),
        QueryMsg::GetOrdersByCreator {
            creator,
            start_after,
            limit,
        } => to_json_binary(&query_orders_by_creator(deps, creator, start_after, limit)?),
        QueryMsg::GetOrderbook {
            base_token,
            quote_token,
            limit,
        } => to_json_binary(&query_orderbook(deps, base_token, quote_token, limit)?),
        QueryMsg::GetBestPrices {
            base_token,
            quote_token,
        } => to_json_binary(&query_best_prices(deps, base_token, quote_token)?),
    }
}

fn query_order(deps: Deps, order_id: u64) -> StdResult<OrderResponse> {
    let order = ORDERS.load(deps.storage, order_id)?;
    Ok(order_to_response(order))
}

fn query_orders_by_creator(
    deps: Deps,
    creator: String,
    _start_after: Option<u64>,
    limit: Option<u32>,
) -> StdResult<OrdersResponse> {
    let creator_addr = deps.api.addr_validate(&creator)?;
    let limit = limit.unwrap_or(10) as usize;

    let orders: Vec<OrderResponse> = CREATOR_ORDERS
        .prefix(&creator_addr)
        .range(deps.storage, None, None, StdOrder::Ascending)
        .take(limit)
        .filter_map(|item| {
            let (order_id, _) = item.ok()?;
            let order = ORDERS.load(deps.storage, order_id).ok()?;
            Some(order_to_response(order))
        })
        .collect();

    Ok(OrdersResponse { orders })
}

fn query_orderbook(
    deps: Deps,
    _base_token: String,
    _quote_token: String,
    _limit: Option<u32>,
) -> StdResult<OrderbookResponse> {
    // Simplified - production would aggregate orders by price level
    Ok(OrderbookResponse {
        bids: vec![],
        asks: vec![],
    })
}

fn query_best_prices(
    _deps: Deps,
    _base_token: String,
    _quote_token: String,
) -> StdResult<BestPricesResponse> {
    // Simplified - production would find highest bid and lowest ask
    Ok(BestPricesResponse {
        best_bid: None,
        best_ask: None,
        spread: None,
    })
}

fn order_to_response(order: Order) -> OrderResponse {
    OrderResponse {
        id: order.id,
        creator: order.creator,
        base_token: order.base_token,
        quote_token: order.quote_token,
        side: order.side,
        base_amount: order.base_amount,
        filled_amount: order.filled_amount,
        price: order.price,
        expiry: order.expiry,
        created_at: order.created_at,
        status: order.status,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::coins;

    #[test]
    fn create_buy_order() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {};
        let info = mock_info("creator", &coins(1000, "earth"));
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let info = mock_info("alice", &[]);
        let msg = ExecuteMsg::CreateBuyOrder {
            base_token: "uatom".to_string(),
            quote_token: "uusd".to_string(),
            base_amount: Uint128::new(100),
            price: Uint128::new(10_000_000_000_000_000_000), // 10.0 USD
            expiry: None,
        };
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(res.attributes.len(), 6);
    }
}
