use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{HumanAddr, Uint128};

use crate::msg::TokenId;

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Cw1155QueryMsg {
    /// Returns the current balance of the given address, 0 if unset.
    /// Return type: BalanceResponse.
    Balance { owner: HumanAddr, token_id: TokenId },
    /// Returns the current balance of the given address for a batch of tokens, 0 if unset.
    /// Return type: BatchBalanceResponse.
    BatchBalance {
        owner: HumanAddr,
        token_ids: Vec<TokenId>,
    },
    /// Query the approval status of an operator for a given owner
    /// Return type: ApprovedForAllResponse.
    ApprovedForAll {
        owner: HumanAddr,
        spender: HumanAddr,
    },
    /// Query metadata of token
    /// Return type: TokenInfoResponse.
    TokenInfo { token_id: TokenId },
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct BalanceResponse {
    pub balance: Uint128,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct BatchBalanceResponse {
    pub balances: Vec<Uint128>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct ApprovedForAllResponse {
    pub approved: bool,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct TokenInfoResponse {
    /// Should be a url point to a json file
    pub url: String,
}
