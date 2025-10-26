use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;
use cw_storage_plus::Map;

#[cw_serde]
pub struct Signer {
    pub address: Addr,
    pub weight: u64,
}

#[cw_serde]
pub struct SignerList {
    /// Owner of the signer list
    pub owner: Addr,
    /// Quorum threshold
    pub quorum: u64,
    /// List of signers with weights
    pub signers: Vec<Signer>,
    /// Total weight
    pub total_weight: u64,
    /// Created timestamp
    pub created_at: u64,
    /// Updated timestamp
    pub updated_at: u64,
}

/// Signer lists indexed by owner
pub const SIGNER_LISTS: Map<&Addr, SignerList> = Map::new("signer_lists");
