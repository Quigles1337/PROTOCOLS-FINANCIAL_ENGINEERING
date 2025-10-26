use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Order not found")]
    OrderNotFound {},

    #[error("Order already filled or cancelled")]
    OrderNotActive {},

    #[error("Invalid amount")]
    InvalidAmount {},

    #[error("Invalid price")]
    InvalidPrice {},

    #[error("Insufficient balance for order")]
    InsufficientBalance {},

    #[error("Order expired")]
    OrderExpired {},

    #[error("Cannot match order with self")]
    SelfMatch {},

    #[error("Order does not cross (price mismatch)")]
    NoCross {},
}
