use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Item, Map};

#[cw_serde]
pub enum OrderSide {
    Buy,
    Sell,
}

#[cw_serde]
pub enum OrderStatus {
    Active,
    PartiallyFilled,
    Filled,
    Cancelled,
}

#[cw_serde]
pub struct Order {
    /// Order unique ID
    pub id: u64,
    /// Order creator
    pub creator: Addr,
    /// Base token (what to buy/sell)
    pub base_token: String,
    /// Quote token (what to pay/receive)
    pub quote_token: String,
    /// Side (buy or sell)
    pub side: OrderSide,
    /// Original base amount
    pub base_amount: Uint128,
    /// Filled base amount
    pub filled_amount: Uint128,
    /// Price (scaled by 1e18, quote per base)
    pub price: Uint128,
    /// Expiration timestamp (0 = never)
    pub expiry: u64,
    /// Created timestamp
    pub created_at: u64,
    /// Status
    pub status: OrderStatus,
}

/// Next order ID
pub const NEXT_ORDER_ID: Item<u64> = Item::new("next_order_id");

/// Orders indexed by ID
pub const ORDERS: Map<u64, Order> = Map::new("orders");

/// Buy orders by trading pair (for matching)
/// Key: "base:quote:price" (price padded for sorting)
pub const BUY_ORDERS: Map<(String, String, String), Vec<u64>> = Map::new("buy_orders");

/// Sell orders by trading pair (for matching)
/// Key: "base:quote:price"
pub const SELL_ORDERS: Map<(String, String, String), Vec<u64>> = Map::new("sell_orders");

/// Orders by creator (for queries)
pub const CREATOR_ORDERS: Map<(&Addr, u64), ()> = Map::new("creator_orders");

/// Helper to format price for storage key (18 decimal places, zero-padded)
pub fn price_key(price: Uint128) -> String {
    format!("{:030}", price.u128())
}
