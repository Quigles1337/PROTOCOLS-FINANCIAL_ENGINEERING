use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Item, Map};

#[cw_serde]
pub enum EscrowStatus {
    Active,
    Released,
    Cancelled,
}

#[cw_serde]
pub struct Escrow {
    /// Escrow unique ID
    pub id: u64,
    /// Sender (who deposits funds)
    pub sender: Addr,
    /// Recipient (who can release funds)
    pub recipient: Addr,
    /// Token denomination
    pub denom: String,
    /// Amount in escrow
    pub amount: Uint128,
    /// Release timestamp (time-lock, 0 = no time lock)
    pub release_time: u64,
    /// Hash for hash-lock (empty = no hash lock)
    pub condition_hash: String,
    /// Expiry timestamp (after which sender can cancel)
    pub expiry_time: u64,
    /// Memo/description
    pub memo: String,
    /// Creation time
    pub created_at: u64,
    /// Status
    pub status: EscrowStatus,
}

/// Next escrow ID
pub const NEXT_ESCROW_ID: Item<u64> = Item::new("next_escrow_id");

/// Escrows indexed by ID
pub const ESCROWS: Map<u64, Escrow> = Map::new("escrows");

/// Escrows by sender (for queries)
pub const SENDER_ESCROWS: Map<(&Addr, u64), ()> = Map::new("sender_escrows");

/// Escrows by recipient (for queries)
pub const RECIPIENT_ESCROWS: Map<(&Addr, u64), ()> = Map::new("recipient_escrows");
