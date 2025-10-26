use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Trust line already exists")]
    TrustLineExists {},

    #[error("Trust line not found")]
    TrustLineNotFound {},

    #[error("Cannot create trust line with self")]
    SelfTrustLine {},

    #[error("Limit must be greater than zero")]
    InvalidLimit {},

    #[error("Insufficient credit available")]
    InsufficientCredit {},

    #[error("Payment amount must be greater than zero")]
    InvalidAmount {},

    #[error("Path cannot be empty")]
    EmptyPath {},

    #[error("Payment path too long (max {max})")]
    PathTooLong { max: usize },

    #[error("Invalid payment path")]
    InvalidPath {},

    #[error("Rippling not enabled on this trust line")]
    RipplingDisabled {},

    #[error("Quality too low for this path")]
    QualityTooLow {},
}
