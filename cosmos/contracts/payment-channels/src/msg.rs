use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Binary, Uint128};
use crate::state::ChannelStatus;

#[cw_serde]
pub struct InstantiateMsg {
    /// Minimum channel duration in seconds
    pub min_duration: Option<u64>,
    /// Maximum channel duration in seconds
    pub max_duration: Option<u64>,
    /// Dispute period in seconds
    pub dispute_period: Option<u64>,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Create a new payment channel
    CreateChannel {
        recipient: String,
        denom: String,
        amount: Uint128,
        duration: u64,
    },
    /// Fund an existing channel
    FundChannel {
        channel_id: u64,
        amount: Uint128,
    },
    /// Extend channel expiration
    ExtendChannel {
        channel_id: u64,
        duration: u64,
    },
    /// Claim payment from channel (with signature)
    ClaimPayment {
        channel_id: u64,
        amount: Uint128,
        nonce: u64,
        signature: Binary,
    },
    /// Close channel cooperatively (both parties agree)
    CloseChannel {
        channel_id: u64,
        final_amount: Uint128,
    },
    /// Close channel unilaterally (after expiration)
    CloseChannelUnilateral {
        channel_id: u64,
    },
    /// Dispute a claim
    DisputeClaim {
        channel_id: u64,
    },
    /// Resolve dispute after dispute period
    ResolveDispute {
        channel_id: u64,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Get configuration
    #[returns(ConfigResponse)]
    Config {},

    /// Get channel by ID
    #[returns(ChannelResponse)]
    GetChannel { channel_id: u64 },

    /// Get channels by sender
    #[returns(ChannelsResponse)]
    GetChannelsBySender {
        sender: String,
        start_after: Option<u64>,
        limit: Option<u32>,
    },

    /// Get channels by recipient
    #[returns(ChannelsResponse)]
    GetChannelsByRecipient {
        recipient: String,
        start_after: Option<u64>,
        limit: Option<u32>,
    },

    /// Get available balance in channel
    #[returns(BalanceResponse)]
    GetAvailableBalance { channel_id: u64 },
}

// Response types

#[cw_serde]
pub struct ConfigResponse {
    pub min_duration: u64,
    pub max_duration: u64,
    pub dispute_period: u64,
}

#[cw_serde]
pub struct ChannelResponse {
    pub id: u64,
    pub sender: Addr,
    pub recipient: Addr,
    pub denom: String,
    pub balance: Uint128,
    pub claimed: Uint128,
    pub nonce: u64,
    pub created_at: u64,
    pub expires_at: u64,
    pub status: ChannelStatus,
    pub disputed_at: Option<u64>,
    pub disputed_amount: Option<Uint128>,
}

#[cw_serde]
pub struct ChannelsResponse {
    pub channels: Vec<ChannelResponse>,
}

#[cw_serde]
pub struct BalanceResponse {
    pub total: Uint128,
    pub claimed: Uint128,
    pub available: Uint128,
}
