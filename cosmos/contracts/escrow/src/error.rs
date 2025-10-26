use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Escrow not found")]
    EscrowNotFound {},

    #[error("Escrow already released")]
    AlreadyReleased {},

    #[error("Escrow already cancelled")]
    AlreadyCancelled {},

    #[error("Amount must be greater than zero")]
    InvalidAmount {},

    #[error("Release time must be in the future")]
    InvalidReleaseTime {},

    #[error("Escrow has not unlocked yet")]
    NotUnlocked {},

    #[error("Escrow has expired")]
    Expired {},

    #[error("Invalid preimage for hashlock")]
    InvalidPreimage {},

    #[error("Cannot cancel time-locked escrow before expiry")]
    CannotCancel {},

    #[error("Invalid condition hash")]
    InvalidConditionHash {},
}
