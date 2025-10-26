use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("DID not found")]
    DIDNotFound {},

    #[error("DID already exists")]
    DIDExists {},

    #[error("Invalid DID URI")]
    InvalidDIDURI {},

    #[error("DID document too large")]
    DocumentTooLarge {},
}
