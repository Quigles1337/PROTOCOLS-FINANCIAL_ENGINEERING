use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Item, Map};

#[cw_serde]
pub enum CheckStatus {
    Active,
    Cashed,
    Cancelled,
}

#[cw_serde]
pub struct Check {
    /// Check unique ID
    pub id: u64,
    /// Sender (check writer)
    pub sender: Addr,
    /// Recipient (payee, optional - if None, anyone can cash)
    pub recipient: Option<Addr>,
    /// Token denomination
    pub denom: String,
    /// Amount
    pub amount: Uint128,
    /// Expiration timestamp (0 = no expiry)
    pub expiry: u64,
    /// Memo
    pub memo: String,
    /// Creation time
    pub created_at: u64,
    /// Status
    pub status: CheckStatus,
    /// Who cashed it (if cashed)
    pub cashed_by: Option<Addr>,
    /// When it was cashed
    pub cashed_at: Option<u64>,
}

/// Next check ID
pub const NEXT_CHECK_ID: Item<u64> = Item::new("next_check_id");

/// Checks indexed by ID
pub const CHECKS: Map<u64, Check> = Map::new("checks");

/// Checks by sender (for queries)
pub const SENDER_CHECKS: Map<(&Addr, u64), ()> = Map::new("sender_checks");

/// Checks by recipient (for queries)
pub const RECIPIENT_CHECKS: Map<(&Addr, u64), ()> = Map::new("recipient_checks");
