use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Binary, HumanAddr, Uint128};
use cw0::Expiration;

pub type TokenId = String;

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Cw1155HandleMsg {
    /// TransferFrom is a base message to move tokens.
    /// if `env.sender` is the owner or has sufficient pre-approval.
    TransferFrom {
        // `None` means minting
        from: Option<HumanAddr>,
        // `None` means burning
        to: Option<HumanAddr>,
        token_id: TokenId,
        value: Uint128,
    },
    /// SendFrom is a base message to move tokens to contract.
    /// if `env.sender` is the owner or has sufficient pre-approval.
    SendFrom {
        // `None` means minting
        from: Option<HumanAddr>,
        contract: HumanAddr,
        token_id: TokenId,
        value: Uint128,
        msg: Option<Binary>,
    },
    /// BatchTransferFrom is a base message to move tokens to another account without triggering actions.
    /// if `env.sender` is the owner or has sufficient pre-approval.
    BatchTransferFrom {
        // `None` means minting
        from: Option<HumanAddr>,
        // `None` means burning
        to: Option<HumanAddr>,
        batch: Vec<(TokenId, Uint128)>,
    },
    /// BatchSendFrom is a base message to move tokens to another to without triggering actions.
    /// if `env.sender` is the owner or has sufficient pre-approval.
    BatchSendFrom {
        // `None` means minting
        from: Option<HumanAddr>,
        contract: HumanAddr,
        batch: Vec<(TokenId, Uint128)>,
        msg: Option<Binary>,
    },
    /// Allows operator to transfer / send any token from the owner's account.
    /// If expiration is set, then this allowance has a time/height limit
    ApproveAll {
        operator: HumanAddr,
        expires: Option<Expiration>,
    },
    /// Remove previously granted ApproveAll permission
    RevokeAll { operator: HumanAddr },
}
