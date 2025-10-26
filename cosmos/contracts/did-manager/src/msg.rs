use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;

#[cw_serde]
pub struct InstantiateMsg {}

#[cw_serde]
pub enum ExecuteMsg {
    /// Set or create DID
    SetDID { did_uri: String, document: String },
    /// Update DID document
    UpdateDID { document: String },
    /// Delete DID
    DeleteDID {},
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Get DID by owner address
    #[returns(DIDResponse)]
    GetDID { owner: String },

    /// Resolve DID URI to owner
    #[returns(ResolveResponse)]
    ResolveDID { did_uri: String },
}

// Response types

#[cw_serde]
pub struct DIDResponse {
    pub owner: Addr,
    pub did_uri: String,
    pub document: String,
    pub created_at: u64,
    pub updated_at: u64,
}

#[cw_serde]
pub struct ResolveResponse {
    pub owner: Addr,
    pub document: String,
}
