use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;

#[cw_serde]
pub struct InstantiateMsg {}

#[cw_serde]
pub enum ExecuteMsg {
    /// Enable deposit authorization
    EnableDepositAuth {},
    /// Disable deposit authorization
    DisableDepositAuth {},
    /// Update settings
    UpdateSettings {
        whitelist_mode: Option<bool>,
        require_auth_for_all_tokens: Option<bool>,
    },
    /// Authorize a depositor
    AuthorizeDepositor { depositor: String },
    /// Unauthorize a depositor
    UnauthorizeDepositor { depositor: String },
    /// Authorize depositor for specific token
    AuthorizeToken {
        depositor: String,
        token: String,
    },
    /// Unauthorize depositor for specific token
    UnauthorizeToken {
        depositor: String,
        token: String,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Get authorization settings
    #[returns(SettingsResponse)]
    GetSettings { account: String },

    /// Check if depositor is authorized
    #[returns(AuthResponse)]
    IsAuthorized {
        account: String,
        depositor: String,
        token: Option<String>,
    },

    /// Get all authorized depositors
    #[returns(AuthorizedListResponse)]
    GetAuthorizedList { account: String },
}

// Response types

#[cw_serde]
pub struct SettingsResponse {
    pub enabled: bool,
    pub whitelist_mode: bool,
    pub require_auth_for_all_tokens: bool,
}

#[cw_serde]
pub struct AuthResponse {
    pub authorized: bool,
    pub reason: String,
}

#[cw_serde]
pub struct AuthorizedListResponse {
    pub depositors: Vec<Addr>,
}
