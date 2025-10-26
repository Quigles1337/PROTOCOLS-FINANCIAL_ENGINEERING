use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;

#[cw_serde]
pub struct InstantiateMsg {
    /// Minimum account age in seconds
    pub min_account_age: Option<u64>,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Create account
    CreateAccount {},
    /// Delete account
    DeleteAccount { beneficiary: String },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Get config
    #[returns(ConfigResponse)]
    GetConfig {},

    /// Get account info
    #[returns(AccountResponse)]
    GetAccount { owner: String },

    /// Check if account is deleted
    #[returns(DeletedResponse)]
    IsDeleted { owner: String },

    /// Check if account can be deleted
    #[returns(CanDeleteResponse)]
    CanDelete { owner: String },
}

// Response types

#[cw_serde]
pub struct ConfigResponse {
    pub min_account_age: u64,
}

#[cw_serde]
pub struct AccountResponse {
    pub owner: Addr,
    pub created_at: u64,
    pub is_deleted: bool,
    pub deleted_at: Option<u64>,
    pub beneficiary: Option<Addr>,
}

#[cw_serde]
pub struct DeletedResponse {
    pub is_deleted: bool,
}

#[cw_serde]
pub struct CanDeleteResponse {
    pub can_delete: bool,
    pub reason: String,
}
