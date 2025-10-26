use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;

#[cw_serde]
pub struct InstantiateMsg {}

#[cw_serde]
pub struct SignerInput {
    pub address: String,
    pub weight: u64,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Set signer list
    SetSignerList {
        quorum: u64,
        signers: Vec<SignerInput>,
    },
    /// Remove signer list
    RemoveSignerList {},
    /// Verify signatures (simplified)
    VerifySignatures {
        signers: Vec<String>,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Get signer list
    #[returns(SignerListResponse)]
    GetSignerList { owner: String },

    /// Check if has signer list
    #[returns(HasListResponse)]
    HasSignerList { owner: String },

    /// Check if signatures meet quorum
    #[returns(QuorumResponse)]
    CheckQuorum {
        owner: String,
        signers: Vec<String>,
    },
}

// Response types

#[cw_serde]
pub struct SignerInfo {
    pub address: Addr,
    pub weight: u64,
}

#[cw_serde]
pub struct SignerListResponse {
    pub owner: Addr,
    pub quorum: u64,
    pub signers: Vec<SignerInfo>,
    pub total_weight: u64,
    pub created_at: u64,
    pub updated_at: u64,
}

#[cw_serde]
pub struct HasListResponse {
    pub has_list: bool,
}

#[cw_serde]
pub struct QuorumResponse {
    pub meets_quorum: bool,
    pub weight: u64,
    pub quorum: u64,
}
