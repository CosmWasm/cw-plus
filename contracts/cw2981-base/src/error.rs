use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("No royalties set for token_id")]
    NoRoyaltiesSet {},

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("token_id already claimed")]
    Claimed {},

    #[error("Cannot set approval that is already expired")]
    Expired {},
}

impl From<cw721_base::ContractError> for ContractError {
    fn from(err: cw721_base::ContractError) -> Self {
        match err {
            cw721_base::ContractError::Std(error) => ContractError::Std(error),
            cw721_base::ContractError::Unauthorized {} => ContractError::Unauthorized {},
            cw721_base::ContractError::Claimed {} => ContractError::Claimed {},
            cw721_base::ContractError::Expired {} => ContractError::Expired {},
        }
    }
}
