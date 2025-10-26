use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Deposit authorization not enabled")]
    NotEnabled {},

    #[error("Depositor not authorized (not on whitelist or on blacklist)")]
    DepositorNotAuthorized {},

    #[error("Depositor already authorized")]
    AlreadyAuthorized {},

    #[error("Depositor already unauthorized")]
    AlreadyUnauthorized {},
}
