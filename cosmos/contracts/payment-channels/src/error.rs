use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Channel not found")]
    ChannelNotFound {},

    #[error("Channel already closed")]
    ChannelClosed {},

    #[error("Channel already exists")]
    ChannelExists {},

    #[error("Invalid recipient")]
    InvalidRecipient {},

    #[error("Amount must be greater than zero")]
    InvalidAmount {},

    #[error("Invalid duration")]
    InvalidDuration {},

    #[error("Invalid nonce (must be greater than previous)")]
    InvalidNonce {},

    #[error("Claim amount exceeds channel balance")]
    InsufficientBalance {},

    #[error("Invalid signature")]
    InvalidSignature {},

    #[error("Channel has not expired yet")]
    ChannelNotExpired {},

    #[error("Dispute period still active")]
    DisputePeriodActive {},

    #[error("No dispute to resolve")]
    NoDispute {},
}
