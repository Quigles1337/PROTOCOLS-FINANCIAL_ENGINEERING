use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Item, Map};

#[cw_serde]
pub enum ChannelStatus {
    Active,
    Disputed,
    Closed,
}

#[cw_serde]
pub struct Channel {
    /// Channel unique ID
    pub id: u64,
    /// Sender (who funds the channel)
    pub sender: Addr,
    /// Recipient (who receives payments)
    pub recipient: Addr,
    /// Token denomination
    pub denom: String,
    /// Total balance in channel
    pub balance: Uint128,
    /// Amount claimed by recipient
    pub claimed: Uint128,
    /// Last nonce used
    pub nonce: u64,
    /// Channel creation time
    pub created_at: u64,
    /// Channel expiration time
    pub expires_at: u64,
    /// Current status
    pub status: ChannelStatus,
    /// Dispute timestamp (if any)
    pub disputed_at: Option<u64>,
    /// Dispute amount (if any)
    pub disputed_amount: Option<Uint128>,
}

#[cw_serde]
pub struct Config {
    /// Minimum channel duration (seconds)
    pub min_duration: u64,
    /// Maximum channel duration (seconds)
    pub max_duration: u64,
    /// Dispute period (seconds)
    pub dispute_period: u64,
}

/// Configuration storage
pub const CONFIG: Item<Config> = Item::new("config");

/// Next channel ID
pub const NEXT_CHANNEL_ID: Item<u64> = Item::new("next_channel_id");

/// Channels indexed by ID
pub const CHANNELS: Map<u64, Channel> = Map::new("channels");

/// Channels by sender (for queries)
pub const SENDER_CHANNELS: Map<(&Addr, u64), ()> = Map::new("sender_channels");

/// Channels by recipient (for queries)
pub const RECIPIENT_CHANNELS: Map<(&Addr, u64), ()> = Map::new("recipient_channels");
