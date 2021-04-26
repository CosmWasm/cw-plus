use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Hash parse error: {0}")]
    ParseError(String),

    #[error("Invalid atomic swap id")]
    InvalidId {},

    #[error("Invalid preimage")]
    InvalidPreimage {},

    #[error("Invalid hash ({0} chars): must be 64 characters")]
    InvalidHash(usize),

    #[error("Send some coins to create an atomic swap")]
    EmptyBalance {},

    #[error("Atomic swap not yet expired")]
    NotExpired,

    #[error("Expired atomic swap")]
    Expired,

    #[error("Atomic swap already exists")]
    AlreadyExists,
}
