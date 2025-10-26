use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};

#[cw_serde]
pub struct Account {
    /// Account owner
    pub owner: Addr,
    /// Created timestamp
    pub created_at: u64,
    /// Is deleted
    pub is_deleted: bool,
    /// Deleted timestamp
    pub deleted_at: Option<u64>,
    /// Beneficiary (who receives remaining funds)
    pub beneficiary: Option<Addr>,
}

#[cw_serde]
pub struct Config {
    /// Minimum account age before deletion (seconds)
    pub min_account_age: u64,
}

/// Configuration
pub const CONFIG: Item<Config> = Item::new("config");

/// Accounts indexed by owner
pub const ACCOUNTS: Map<&Addr, Account> = Map::new("accounts");
