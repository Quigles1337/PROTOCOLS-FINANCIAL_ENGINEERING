use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Uint128};
use crate::state::EscrowStatus;

#[cw_serde]
pub struct InstantiateMsg {}

#[cw_serde]
pub enum ExecuteMsg {
    /// Create time-locked escrow
    CreateTimeLock {
        recipient: String,
        denom: String,
        amount: Uint128,
        release_time: u64,
        expiry_time: u64,
        memo: Option<String>,
    },
    /// Create hash-locked escrow (HTLC)
    CreateHashLock {
        recipient: String,
        denom: String,
        amount: Uint128,
        condition_hash: String,
        expiry_time: u64,
        memo: Option<String>,
    },
    /// Create time + hash locked escrow
    CreateTimedHashLock {
        recipient: String,
        denom: String,
        amount: Uint128,
        release_time: u64,
        condition_hash: String,
        expiry_time: u64,
        memo: Option<String>,
    },
    /// Release escrow (recipient)
    Release {
        escrow_id: u64,
        preimage: Option<String>,
    },
    /// Cancel escrow (sender, only after expiry or if not time-locked)
    Cancel { escrow_id: u64 },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Get escrow by ID
    #[returns(EscrowResponse)]
    GetEscrow { escrow_id: u64 },

    /// Get escrows by sender
    #[returns(EscrowsResponse)]
    GetEscrowsBySender {
        sender: String,
        start_after: Option<u64>,
        limit: Option<u32>,
    },

    /// Get escrows by recipient
    #[returns(EscrowsResponse)]
    GetEscrowsByRecipient {
        recipient: String,
        start_after: Option<u64>,
        limit: Option<u32>,
    },

    /// Check if escrow is unlocked
    #[returns(UnlockStatusResponse)]
    IsUnlocked { escrow_id: u64 },
}

// Response types

#[cw_serde]
pub struct EscrowResponse {
    pub id: u64,
    pub sender: Addr,
    pub recipient: Addr,
    pub denom: String,
    pub amount: Uint128,
    pub release_time: u64,
    pub condition_hash: String,
    pub expiry_time: u64,
    pub memo: String,
    pub created_at: u64,
    pub status: EscrowStatus,
}

#[cw_serde]
pub struct EscrowsResponse {
    pub escrows: Vec<EscrowResponse>,
}

#[cw_serde]
pub struct UnlockStatusResponse {
    pub unlocked: bool,
    pub time_locked: bool,
    pub hash_locked: bool,
    pub time_remaining: Option<u64>,
}
