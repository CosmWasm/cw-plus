use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Binary, HumanAddr, Uint128};

pub type TokenId = String;

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Cw1155HandleMsg {
    /// TransferFrom is a base message to move tokens
    /// if `env.sender` has sufficient pre-approval.
    TransferFrom {
        from: Option<HumanAddr>,
        to: Option<HumanAddr>,
        token_id: TokenId,
        value: Uint128,
    },
    /// SendFrom is a base message to move tokens to contract
    /// if `env.sender` has sufficient pre-approval.
    SendFrom {
        from: Option<HumanAddr>,
        contract: HumanAddr,
        token_id: TokenId,
        value: Uint128,
        msg: Option<Binary>,
    },
    /// BatchTransferFrom is a base message to move tokens to another account without triggering actions
    BatchTransferFrom {
        from: HumanAddr,
        to: HumanAddr,
        batch: Vec<(TokenId, Uint128)>,
    },
    /// BatchSendFrom is a base message to move tokens to another to without triggering actions
    BatchSendFrom {
        from: HumanAddr,
        contract: HumanAddr,
        batch: Vec<(TokenId, Uint128)>,
        msg: Option<Binary>,
    },
    /// Enable or disable approval for a third party ("operator") to manage all of the caller's tokens.
    SetApprovalForAll { operator: HumanAddr, approved: bool },
}
