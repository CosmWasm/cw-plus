use cosmwasm_std::StdError;
use cw_utils::Expiration;
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

    #[error("Allowance already expired while setting: {0}")]
    SettingExpiredAllowance(Expiration),

    #[error("Semver parsing error: {0}")]
    SemVer(String),
}

impl From<cw1_whitelist::ContractError> for ContractError {
    fn from(err: cw1_whitelist::ContractError) -> Self {
        match err {
            cw1_whitelist::ContractError::Std(error) => ContractError::Std(error),
            cw1_whitelist::ContractError::Unauthorized {} => ContractError::Unauthorized {},
        }
    }
}

impl From<semver::Error> for ContractError {
    fn from(err: semver::Error) -> Self {
        Self::SemVer(err.to_string())
    }
}
