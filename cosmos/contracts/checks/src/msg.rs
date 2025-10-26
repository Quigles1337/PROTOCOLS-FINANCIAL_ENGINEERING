use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Uint128};
use crate::state::CheckStatus;

#[cw_serde]
pub struct InstantiateMsg {}

#[cw_serde]
pub enum ExecuteMsg {
    /// Create a check
    CreateCheck {
        recipient: Option<String>,
        denom: String,
        amount: Uint128,
        expiry: Option<u64>,
        memo: Option<String>,
    },
    /// Cash a check
    CashCheck { check_id: u64 },
    /// Cancel a check (sender only)
    CancelCheck { check_id: u64 },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Get check by ID
    #[returns(CheckResponse)]
    GetCheck { check_id: u64 },

    /// Get checks by sender
    #[returns(ChecksResponse)]
    GetChecksBySender {
        sender: String,
        start_after: Option<u64>,
        limit: Option<u32>,
    },

    /// Get checks by recipient
    #[returns(ChecksResponse)]
    GetChecksByRecipient {
        recipient: String,
        start_after: Option<u64>,
        limit: Option<u32>,
    },

    /// Get check status
    #[returns(CheckStatusResponse)]
    GetCheckStatus { check_id: u64 },
}

// Response types

#[cw_serde]
pub struct CheckResponse {
    pub id: u64,
    pub sender: Addr,
    pub recipient: Option<Addr>,
    pub denom: String,
    pub amount: Uint128,
    pub expiry: u64,
    pub memo: String,
    pub created_at: u64,
    pub status: CheckStatus,
    pub cashed_by: Option<Addr>,
    pub cashed_at: Option<u64>,
}

#[cw_serde]
pub struct ChecksResponse {
    pub checks: Vec<CheckResponse>,
}

#[cw_serde]
pub struct CheckStatusResponse {
    pub status: CheckStatus,
    pub can_cash: bool,
    pub is_expired: bool,
}
