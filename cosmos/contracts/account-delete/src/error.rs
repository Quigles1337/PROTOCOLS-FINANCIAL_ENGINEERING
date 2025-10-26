use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Account not found")]
    AccountNotFound {},

    #[error("Account already deleted")]
    AlreadyDeleted {},

    #[error("Account too new to delete (minimum age: {min_age} seconds)")]
    AccountTooNew { min_age: u64 },

    #[error("Account has outstanding balance")]
    HasBalance {},
}
