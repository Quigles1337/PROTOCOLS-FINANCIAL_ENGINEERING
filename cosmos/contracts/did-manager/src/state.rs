use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;
use cw_storage_plus::Map;

#[cw_serde]
pub struct DIDDocument {
    /// DID owner
    pub owner: Addr,
    /// DID URI (did:cosmos:...)
    pub did_uri: String,
    /// DID Document (JSON)
    pub document: String,
    /// Created timestamp
    pub created_at: u64,
    /// Updated timestamp
    pub updated_at: u64,
}

/// DIDs indexed by owner address
pub const DIDS: Map<&Addr, DIDDocument> = Map::new("dids");

/// Reverse lookup: DID URI to owner address
pub const DID_REVERSE: Map<String, Addr> = Map::new("did_reverse");
