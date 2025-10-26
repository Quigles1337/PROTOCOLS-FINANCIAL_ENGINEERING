use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Uint128};

#[cw_serde]
pub struct InstantiateMsg {
    /// Maximum path length for rippling payments
    pub max_path_length: Option<usize>,
    /// Minimum quality for paths (scaled by 1e9)
    pub min_quality: Option<u64>,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Create a new trust line
    CreateTrustLine {
        counterparty: String,
        denom: String,
        limit: Uint128,
        allow_rippling: Option<bool>,
        quality_in: Option<u64>,
        quality_out: Option<u64>,
    },
    /// Update an existing trust line
    UpdateTrustLine {
        counterparty: String,
        denom: String,
        limit: Option<Uint128>,
        allow_rippling: Option<bool>,
        quality_in: Option<u64>,
        quality_out: Option<u64>,
    },
    /// Close a trust line (must have zero balance)
    CloseTrustLine {
        counterparty: String,
        denom: String,
    },
    /// Send payment directly through a trust line
    SendPayment {
        recipient: String,
        denom: String,
        amount: Uint128,
    },
    /// Send payment through a path of trust lines (rippling)
    SendPaymentThroughPath {
        recipient: String,
        denom: String,
        amount: Uint128,
        path: Vec<String>,
        max_quality_loss: Option<u64>,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Get configuration
    #[returns(ConfigResponse)]
    Config {},

    /// Get a specific trust line
    #[returns(TrustLineResponse)]
    GetTrustLine {
        account1: String,
        account2: String,
        denom: String,
    },

    /// Get all trust lines for an account
    #[returns(TrustLinesResponse)]
    GetTrustLines {
        account: String,
        start_after: Option<String>,
        limit: Option<u32>,
    },

    /// Get available credit between two accounts
    #[returns(CreditResponse)]
    GetAvailableCredit {
        from: String,
        to: String,
        denom: String,
    },

    /// Find payment path between two accounts
    #[returns(PathResponse)]
    FindPath {
        from: String,
        to: String,
        denom: String,
        amount: Uint128,
    },
}

// Response types

#[cw_serde]
pub struct ConfigResponse {
    pub max_path_length: usize,
    pub min_quality: u64,
}

#[cw_serde]
pub struct TrustLineResponse {
    pub account1: Addr,
    pub account2: Addr,
    pub denom: String,
    pub limit1: Uint128,
    pub limit2: Uint128,
    pub balance: i128,
    pub allow_rippling: bool,
    pub quality_in: u64,
    pub quality_out: u64,
    pub created_at: u64,
    pub updated_at: u64,
}

#[cw_serde]
pub struct TrustLinesResponse {
    pub trust_lines: Vec<TrustLineResponse>,
}

#[cw_serde]
pub struct CreditResponse {
    pub available: Uint128,
    pub limit: Uint128,
    pub used: Uint128,
}

#[cw_serde]
pub struct PathResponse {
    pub path: Vec<Addr>,
    pub quality: u64,
    pub found: bool,
}
