use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Invalid {0}")]
    Invalid(String),

    #[error("{0}")]
    EmptyBalance(String),

    #[error("Atomic swap not yet expired")]
    NotExpired,

    #[error("Expired atomic swap")]
    Expired,

    #[error("Atomic swap already exists")]
    AlreadyExists,
}
