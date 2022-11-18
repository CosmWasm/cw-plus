use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Supported Interface must be more than zero")]
    SupportedInterfaceMustBeMoreThanZero {},

    #[error("Contract does not support this interface")]
    ContractDoesNotSupportThisInterface {},
}
