#![no_std]

//! DEXOrders - On-Chain Orderbook with Automatic Matching
//! Production-grade Soroban implementation

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype,
    token, Address, Env, Vec, vec,
};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OrderSide {
    Buy,
    Sell,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OrderStatus {
    Open,
    PartiallyFilled,
    Filled,
    Cancelled,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Order {
    pub id: u64,
    pub trader: Address,
    pub side: OrderSide,
    pub base_token: Address,
    pub quote_token: Address,
    pub price: i128,
    pub amount: i128,
    pub filled: i128,
    pub status: OrderStatus,
    pub created_at: u64,
}

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Order(u64),
    NextOrderId,
    BuyOrders(Address, Address),
    SellOrders(Address, Address),
    Admin,
    FeeRate,
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    NotFound = 1,
    Unauthorized = 2,
    InvalidAmount = 3,
    InvalidPrice = 4,
    OrderNotOpen = 5,
    InsufficientFunds = 6,
}

#[contract]
pub struct DEXOrdersContract;

#[contractimpl]
impl DEXOrdersContract {
    pub fn initialize(env: Env, admin: Address, fee_rate: i128) {
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::NextOrderId, &1u64);
        env.storage().instance().set(&DataKey::FeeRate, &fee_rate);
    }

    pub fn create_buy_order(
        env: Env,
        base_token: Address,
        quote_token: Address,
        price: i128,
        amount: i128,
    ) -> Result<u64, Error> {
        Self::create_order_internal(env, OrderSide::Buy, base_token, quote_token, price, amount)
    }

    pub fn create_sell_order(
        env: Env,
        base_token: Address,
        quote_token: Address,
        price: i128,
        amount: i128,
    ) -> Result<u64, Error> {
        Self::create_order_internal(env, OrderSide::Sell, base_token, quote_token, price, amount)
    }

    fn create_order_internal(
        env: Env,
        side: OrderSide,
        base_token: Address,
        quote_token: Address,
        price: i128,
        amount: i128,
    ) -> Result<u64, Error> {
        let trader = env.invoker();
        trader.require_auth();

        if amount <= 0 { return Err(Error::InvalidAmount); }
        if price <= 0 { return Err(Error::InvalidPrice); }

        let required_funds = match side {
            OrderSide::Buy => price.checked_mul(amount).ok_or(Error::InvalidAmount)?.checked_div(1_000_000).ok_or(Error::InvalidAmount)?,
            OrderSide::Sell => amount,
        };

        let deposit_token = match side { OrderSide::Buy => &quote_token, OrderSide::Sell => &base_token };
        let token_client = token::Client::new(&env, deposit_token);
        token_client.transfer(&trader, &env.current_contract_address(), &required_funds);

        let order_id: u64 = env.storage().instance().get(&DataKey::NextOrderId).unwrap_or(1);
        env.storage().instance().set(&DataKey::NextOrderId, &(order_id + 1));

        let order = Order {
            id: order_id, trader: trader.clone(), side: side.clone(),
            base_token: base_token.clone(), quote_token: quote_token.clone(),
            price, amount, filled: 0, status: OrderStatus::Open,
            created_at: env.ledger().timestamp(),
        };

        env.storage().persistent().set(&DataKey::Order(order_id), &order);
        env.storage().persistent().extend_ttl(&DataKey::Order(order_id), 518400, 518400);

        let orders_key = match side {
            OrderSide::Buy => DataKey::BuyOrders(base_token.clone(), quote_token.clone()),
            OrderSide::Sell => DataKey::SellOrders(base_token.clone(), quote_token.clone()),
        };

        let mut orders: Vec<u64> = env.storage().persistent().get(&orders_key).unwrap_or(vec![&env]);
        orders.push_back(order_id);
        env.storage().persistent().set(&orders_key, &orders);

        env.events().publish((symbol_short!("order"), trader, side), (order_id, price, amount));
        Self::try_match_order(env, order_id)?;
        Ok(order_id)
    }

    fn try_match_order(env: Env, order_id: u64) -> Result<(), Error> {
        let order: Order = env.storage().persistent().get(&DataKey::Order(order_id)).ok_or(Error::NotFound)?;
        if !matches!(order.status, OrderStatus::Open | OrderStatus::PartiallyFilled) { return Ok(()); }

        let opposite_key = match order.side {
            OrderSide::Buy => DataKey::SellOrders(order.base_token.clone(), order.quote_token.clone()),
            OrderSide::Sell => DataKey::BuyOrders(order.base_token.clone(), order.quote_token.clone()),
        };

        let opposite_orders: Vec<u64> = env.storage().persistent().get(&opposite_key).unwrap_or(vec![&env]);

        for opp_id in opposite_orders.iter() {
            let mut opp_order: Order = match env.storage().persistent().get(&DataKey::Order(opp_id)) {
                Some(o) => o,
                None => continue,
            };

            if !matches!(opp_order.status, OrderStatus::Open | OrderStatus::PartiallyFilled) { continue; }

            let can_match = match order.side {
                OrderSide::Buy => order.price >= opp_order.price,
                OrderSide::Sell => order.price <= opp_order.price,
            };

            if !can_match { continue; }

            let mut current_order = order.clone();
            let remaining_amount = current_order.amount - current_order.filled;
            let opp_remaining = opp_order.amount - opp_order.filled;
            let fill_amount = remaining_amount.min(opp_remaining);

            if fill_amount <= 0 { break; }

            Self::execute_trade(env.clone(), &mut current_order, &mut opp_order, fill_amount, opp_order.price)?;
            env.storage().persistent().set(&DataKey::Order(current_order.id), &current_order);
            env.storage().persistent().set(&DataKey::Order(opp_order.id), &opp_order);

            if current_order.filled >= current_order.amount { break; }
        }
        Ok(())
    }

    fn execute_trade(env: Env, order1: &mut Order, order2: &mut Order, amount: i128, exec_price: i128) -> Result<(), Error> {
        let quote_amount = exec_price.checked_mul(amount).ok_or(Error::InvalidAmount)?.checked_div(1_000_000).ok_or(Error::InvalidAmount)?;
        let (buyer, seller) = match order1.side { OrderSide::Buy => (order1, order2), OrderSide::Sell => (order2, order1) };

        let base_token_client = token::Client::new(&env, &buyer.base_token);
        let quote_token_client = token::Client::new(&env, &buyer.quote_token);

        base_token_client.transfer(&env.current_contract_address(), &buyer.trader, &amount);
        quote_token_client.transfer(&env.current_contract_address(), &seller.trader, &quote_amount);

        order1.filled = order1.filled.checked_add(amount).ok_or(Error::InvalidAmount)?;
        order2.filled = order2.filled.checked_add(amount).ok_or(Error::InvalidAmount)?;

        order1.status = if order1.filled >= order1.amount { OrderStatus::Filled } else { OrderStatus::PartiallyFilled };
        order2.status = if order2.filled >= order2.amount { OrderStatus::Filled } else { OrderStatus::PartiallyFilled };

        env.events().publish((symbol_short!("trade"), order1.id, order2.id), (amount, exec_price));
        Ok(())
    }

    pub fn cancel_order(env: Env, order_id: u64) -> Result<(), Error> {
        let caller = env.invoker();
        caller.require_auth();

        let mut order: Order = env.storage().persistent().get(&DataKey::Order(order_id)).ok_or(Error::NotFound)?;
        if caller != order.trader { return Err(Error::Unauthorized); }
        if !matches!(order.status, OrderStatus::Open | OrderStatus::PartiallyFilled) { return Err(Error::OrderNotOpen); }

        let remaining = order.amount.checked_sub(order.filled).ok_or(Error::InvalidAmount)?;

        if remaining > 0 {
            let refund_amount = match order.side {
                OrderSide::Buy => order.price.checked_mul(remaining).ok_or(Error::InvalidAmount)?.checked_div(1_000_000).ok_or(Error::InvalidAmount)?,
                OrderSide::Sell => remaining,
            };

            let refund_token = match order.side { OrderSide::Buy => &order.quote_token, OrderSide::Sell => &order.base_token };
            let token_client = token::Client::new(&env, refund_token);
            token_client.transfer(&env.current_contract_address(), &order.trader, &refund_amount);
        }

        order.status = OrderStatus::Cancelled;
        env.storage().persistent().set(&DataKey::Order(order_id), &order);
        env.events().publish((symbol_short!("cancel"), order_id), ());
        Ok(())
    }

    pub fn get_order(env: Env, order_id: u64) -> Option<Order> {
        env.storage().persistent().get(&DataKey::Order(order_id))
    }

    pub fn get_buy_orders(env: Env, base_token: Address, quote_token: Address) -> Vec<u64> {
        env.storage().persistent().get(&DataKey::BuyOrders(base_token, quote_token)).unwrap_or(vec![&env])
    }

    pub fn get_sell_orders(env: Env, base_token: Address, quote_token: Address) -> Vec<u64> {
        env.storage().persistent().get(&DataKey::SellOrders(base_token, quote_token)).unwrap_or(vec![&env])
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Address, Env};

    #[test]
    fn test_order_creation() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, DEXOrdersContract);
        let client = DEXOrdersContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        client.initialize(&admin, &100);

        let base = Address::generate(&env);
        let quote = Address::generate(&env);

        let buy_id = client.create_buy_order(&base, &quote, &1_000_000, &100);
        let order = client.get_order(&buy_id).unwrap();
        assert_eq!(order.status, OrderStatus::Open);
        assert_eq!(order.amount, 100);
    }

    #[test]
    fn test_order_matching() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, DEXOrdersContract);
        let client = DEXOrdersContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        client.initialize(&admin, &100);

        let base = Address::generate(&env);
        let quote = Address::generate(&env);

        let sell_id = client.create_sell_order(&base, &quote, &1_000_000, &50);
        let buy_id = client.create_buy_order(&base, &quote, &1_000_000, &50);

        let sell_order = client.get_order(&sell_id).unwrap();
        let buy_order = client.get_order(&buy_id).unwrap();

        assert_eq!(sell_order.status, OrderStatus::Filled);
        assert_eq!(buy_order.status, OrderStatus::Filled);
    }

    #[test]
    fn test_partial_fill() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, DEXOrdersContract);
        let client = DEXOrdersContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        client.initialize(&admin, &100);

        let base = Address::generate(&env);
        let quote = Address::generate(&env);

        let sell_id = client.create_sell_order(&base, &quote, &1_000_000, &100);
        let buy_id = client.create_buy_order(&base, &quote, &1_000_000, &50);

        let sell_order = client.get_order(&sell_id).unwrap();
        assert_eq!(sell_order.status, OrderStatus::PartiallyFilled);
        assert_eq!(sell_order.filled, 50);
    }

    #[test]
    fn test_cancel_order() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, DEXOrdersContract);
        let client = DEXOrdersContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        client.initialize(&admin, &100);

        let base = Address::generate(&env);
        let quote = Address::generate(&env);

        let order_id = client.create_buy_order(&base, &quote, &1_000_000, &100);
        client.cancel_order(&order_id);

        let order = client.get_order(&order_id).unwrap();
        assert_eq!(order.status, OrderStatus::Cancelled);
    }
}
