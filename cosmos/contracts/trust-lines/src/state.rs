use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Item, Map};

#[cw_serde]
pub struct TrustLine {
    /// Account 1 (lower address for consistent ordering)
    pub account1: Addr,
    /// Account 2 (higher address for consistent ordering)
    pub account2: Addr,
    /// Token denomination
    pub denom: String,
    /// Credit limit from account1 to account2
    pub limit1: Uint128,
    /// Credit limit from account2 to account1
    pub limit2: Uint128,
    /// Current balance (positive = account2 owes account1)
    pub balance: i128,
    /// Allow rippling through this trust line
    pub allow_rippling: bool,
    /// Quality in (for DEX integration, scaled by 1e9)
    pub quality_in: u64,
    /// Quality out (for DEX integration, scaled by 1e9)
    pub quality_out: u64,
    /// Creation timestamp
    pub created_at: u64,
    /// Last update timestamp
    pub updated_at: u64,
}

#[cw_serde]
pub struct Config {
    /// Maximum path length for rippling
    pub max_path_length: usize,
    /// Minimum quality (scaled by 1e9)
    pub min_quality: u64,
}

/// Configuration storage
pub const CONFIG: Item<Config> = Item::new("config");

/// Trust lines indexed by (account1, account2, denom)
/// Key format: "{account1}:{account2}:{denom}"
pub const TRUST_LINES: Map<String, TrustLine> = Map::new("trust_lines");

/// Helper to create consistent trust line key
pub fn trust_line_key(account1: &Addr, account2: &Addr, denom: &str) -> String {
    let (a1, a2) = if account1 < account2 {
        (account1, account2)
    } else {
        (account2, account1)
    };
    format!("{}:{}:{}", a1, a2, denom)
}
