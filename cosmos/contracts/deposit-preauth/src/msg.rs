use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Uint128};

#[cw_serde]
pub struct InstantiateMsg {}

#[cw_serde]
pub enum ExecuteMsg {
    /// Preauthorize a deposit
    PreauthorizeDeposit {
        authorized: String,
        denom: String,
        max_amount: Uint128,
        expires_at: Option<u64>,
        memo: Option<String>,
    },
    /// Revoke a preauthorization
    RevokePreauth {
        authorized: String,
        denom: String,
    },
    /// Use a preauthorization (mark as used)
    UsePreauth {
        authorizer: String,
        denom: String,
        amount: Uint128,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Get preauthorization
    #[returns(PreauthResponse)]
    GetPreauth {
        authorizer: String,
        authorized: String,
        denom: String,
    },

    /// Get all preauths by authorizer
    #[returns(PreauthsResponse)]
    GetPreauthsByAuthorizer { authorizer: String },

    /// Get all preauths for authorized
    #[returns(PreauthsResponse)]
    GetPreauthsByAuthorized { authorized: String },

    /// Check if preauth is valid
    #[returns(ValidResponse)]
    IsValid {
        authorizer: String,
        authorized: String,
        denom: String,
        amount: Uint128,
    },
}

// Response types

#[cw_serde]
pub struct PreauthResponse {
    pub authorizer: Addr,
    pub authorized: Addr,
    pub denom: String,
    pub max_amount: Uint128,
    pub expires_at: u64,
    pub is_used: bool,
    pub is_revoked: bool,
    pub created_at: u64,
    pub memo: String,
}

#[cw_serde]
pub struct PreauthsResponse {
    pub preauths: Vec<PreauthResponse>,
}

#[cw_serde]
pub struct ValidResponse {
    pub valid: bool,
    pub reason: String,
}
