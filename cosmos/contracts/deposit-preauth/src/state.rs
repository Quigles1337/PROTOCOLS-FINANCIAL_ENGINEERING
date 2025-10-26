use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::Map;

#[cw_serde]
pub struct Preauth {
    /// Authorizer (who grants permission)
    pub authorizer: Addr,
    /// Authorized (who can deposit)
    pub authorized: Addr,
    /// Token denomination
    pub denom: String,
    /// Maximum amount allowed
    pub max_amount: Uint128,
    /// Expiration timestamp (0 = never)
    pub expires_at: u64,
    /// Has been used
    pub is_used: bool,
    /// Has been revoked
    pub is_revoked: bool,
    /// Creation timestamp
    pub created_at: u64,
    /// Memo
    pub memo: String,
}

/// Preauthorizations indexed by (authorizer, authorized, denom)
pub const PREAUTHS: Map<(&Addr, &Addr, &str), Preauth> = Map::new("preauths");
