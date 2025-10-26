use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Preauthorization not found")]
    PreauthNotFound {},

    #[error("Preauthorization already used")]
    AlreadyUsed {},

    #[error("Preauthorization has been revoked")]
    Revoked {},

    #[error("Preauthorization has expired")]
    Expired {},

    #[error("Amount exceeds preauthorized maximum")]
    ExceedsMax {},
}
