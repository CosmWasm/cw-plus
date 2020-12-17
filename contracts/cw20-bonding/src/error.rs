use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Base(#[from] cw20_base::ContractError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Must send reserve token '{0}'")]
    MissingDenom(String),

    #[error("Sent unsupported token, must send reserve token '{0}'")]
    ExtraDenoms(String),

    #[error("No funds sent")]
    NoFunds {},
}
