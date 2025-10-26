use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Uint128};
use crate::state::{OrderSide, OrderStatus};

#[cw_serde]
pub struct InstantiateMsg {}

#[cw_serde]
pub enum ExecuteMsg {
    /// Create a buy order
    CreateBuyOrder {
        base_token: String,
        quote_token: String,
        base_amount: Uint128,
        price: Uint128,
        expiry: Option<u64>,
    },
    /// Create a sell order
    CreateSellOrder {
        base_token: String,
        quote_token: String,
        base_amount: Uint128,
        price: Uint128,
        expiry: Option<u64>,
    },
    /// Cancel an order
    CancelOrder { order_id: u64 },
    /// Fill an order (market taker)
    FillOrder {
        order_id: u64,
        amount: Uint128,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Get order by ID
    #[returns(OrderResponse)]
    GetOrder { order_id: u64 },

    /// Get orders by creator
    #[returns(OrdersResponse)]
    GetOrdersByCreator {
        creator: String,
        start_after: Option<u64>,
        limit: Option<u32>,
    },

    /// Get orderbook for a trading pair
    #[returns(OrderbookResponse)]
    GetOrderbook {
        base_token: String,
        quote_token: String,
        limit: Option<u32>,
    },

    /// Get best bid/ask
    #[returns(BestPricesResponse)]
    GetBestPrices {
        base_token: String,
        quote_token: String,
    },
}

// Response types

#[cw_serde]
pub struct OrderResponse {
    pub id: u64,
    pub creator: Addr,
    pub base_token: String,
    pub quote_token: String,
    pub side: OrderSide,
    pub base_amount: Uint128,
    pub filled_amount: Uint128,
    pub price: Uint128,
    pub expiry: u64,
    pub created_at: u64,
    pub status: OrderStatus,
}

#[cw_serde]
pub struct OrdersResponse {
    pub orders: Vec<OrderResponse>,
}

#[cw_serde]
pub struct OrderbookLevel {
    pub price: Uint128,
    pub amount: Uint128,
    pub order_count: u32,
}

#[cw_serde]
pub struct OrderbookResponse {
    pub bids: Vec<OrderbookLevel>,
    pub asks: Vec<OrderbookLevel>,
}

#[cw_serde]
pub struct BestPricesResponse {
    pub best_bid: Option<Uint128>,
    pub best_ask: Option<Uint128>,
    pub spread: Option<Uint128>,
}
