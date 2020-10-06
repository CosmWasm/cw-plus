use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Required weight cannot be zero")]
    ZeroWeight {},

    #[error("Not possible to reach required (passing) weight")]
    UnreachableWeight {},

    #[error("No voters")]
    NoVoters {},

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Proposal is not open")]
    NotOpen {},

    #[error("Proposal voting period has expired")]
    Expired {},

    #[error("Proposal must expire before you can close it")]
    NotExpired {},

    #[error("Wrong expiration option")]
    WrongExpiration {},

    #[error("Already voted on this proposal")]
    AlreadyVoted {},

    #[error("Proposal must have passed and not yet been executed")]
    WrongExecuteStatus {},

    #[error("Cannot close completed or passed proposals")]
    WrongCloseStatus {},
}
