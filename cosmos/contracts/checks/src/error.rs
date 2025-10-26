use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Check not found")]
    CheckNotFound {},

    #[error("Check already cashed")]
    AlreadyCashed {},

    #[error("Check already cancelled")]
    AlreadyCancelled {},

    #[error("Check has expired")]
    CheckExpired {},

    #[error("Amount must be greater than zero")]
    InvalidAmount {},

    #[error("Cannot cash check before cashes cannot cash your own check")]
    CannotCashOwnCheck {},

    #[error("Insufficient funds to create check")]
    InsufficientFunds {},

    #[error("Invalid recipient")]
    InvalidRecipient {},
}
