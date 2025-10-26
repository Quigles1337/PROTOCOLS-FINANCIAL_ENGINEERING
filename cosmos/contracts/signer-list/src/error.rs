use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Signer list not found")]
    SignerListNotFound {},

    #[error("Invalid quorum (must be between 1 and total weight)")]
    InvalidQuorum {},

    #[error("Signers and weights length mismatch")]
    LengthMismatch {},

    #[error("Empty signer list")]
    EmptySignerList {},

    #[error("Duplicate signer")]
    DuplicateSigner {},

    #[error("Insufficient weight (need {quorum}, have {weight})")]
    InsufficientWeight { quorum: u64, weight: u64 },
}
