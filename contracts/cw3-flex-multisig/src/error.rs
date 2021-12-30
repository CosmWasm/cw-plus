use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Invalid voting threshold percentage, must be in the 0.5-1.0 range")]
    InvalidThreshold {},

    #[error("Required quorum threshold cannot be zero")]
    ZeroQuorumThreshold {},

    #[error("Not possible to reach required quorum threshold")]
    UnreachableQuorumThreshold {},

    #[error("Required weight cannot be zero")]
    ZeroWeight {},

    #[error("Not possible to reach required (passing) weight")]
    UnreachableWeight {},

    #[error("Group contract invalid address '{addr}'")]
    InvalidGroup { addr: String },

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
