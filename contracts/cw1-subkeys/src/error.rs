use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Cannot set to own account")]
    CannotSetOwnAccount {},

    #[error("No permissions for this account")]
    NotAllowed {},

    #[error("No allowance for this account")]
    NoAllowance {},

    #[error("Message type rejected")]
    MessageTypeRejected {},

    #[error("Delegate is not allowed")]
    DelegatePerm {},

    #[error("Re-delegate is not allowed")]
    ReDelegatePerm {},

    #[error("Un-delegate is not allowed")]
    UnDelegatePerm {},

    #[error("Withdraw is not allowed")]
    WithdrawPerm {},

    #[error("Set withdraw address is not allowed")]
    WithdrawAddrPerm {},

    #[error("Unsupported message")]
    UnsupportedMessage {},
}

impl From<cw1_whitelist::ContractError> for ContractError {
    fn from(err: cw1_whitelist::ContractError) -> Self {
        match err {
            cw1_whitelist::ContractError::Std(error) => ContractError::Std(error),
            cw1_whitelist::ContractError::Unauthorized {} => ContractError::Unauthorized {},
        }
    }
}
