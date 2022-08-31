use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Uint128;
use cw_utils::Expiration;

use crate::msg::TokenId;

#[cw_serde]
#[derive(QueryResponses)]
pub enum Cw1155QueryMsg {
    /// Returns the current balance of the given address, 0 if unset.
    #[returns(BalanceResponse)]
    Balance { owner: String, token_id: TokenId },
    /// Returns the current balance of the given address for a batch of tokens, 0 if unset.
    #[returns(BatchBalanceResponse)]
    BatchBalance {
        owner: String,
        token_ids: Vec<TokenId>,
    },
    /// List all operators that can access all of the owner's tokens.
    #[returns(ApprovedForAllResponse)]
    ApprovedForAll {
        owner: String,
        /// unset or false will filter out expired approvals, you must set to true to see them
        include_expired: Option<bool>,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// Query approved status `owner` granted to `operator`.
    #[returns(IsApprovedForAllResponse)]
    IsApprovedForAll { owner: String, operator: String },

    /// With MetaData Extension.
    /// Query metadata of token
    #[returns(TokenInfoResponse)]
    TokenInfo { token_id: TokenId },

    /// With Enumerable extension.
    /// Returns all tokens owned by the given address, [] if unset.
    #[returns(TokensResponse)]
    Tokens {
        owner: String,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// With Enumerable extension.
    /// Requires pagination. Lists all token_ids controlled by the contract.
    #[returns(TokensResponse)]
    AllTokens {
        start_after: Option<String>,
        limit: Option<u32>,
    },
}

#[cw_serde]
pub struct BalanceResponse {
    pub balance: Uint128,
}

#[cw_serde]
pub struct BatchBalanceResponse {
    pub balances: Vec<Uint128>,
}

#[cw_serde]
pub struct Approval {
    /// Account that can transfer/send the token
    pub spender: String,
    /// When the Approval expires (maybe Expiration::never)
    pub expires: Expiration,
}

#[cw_serde]
pub struct ApprovedForAllResponse {
    pub operators: Vec<Approval>,
}

#[cw_serde]
pub struct IsApprovedForAllResponse {
    pub approved: bool,
}

#[cw_serde]
pub struct TokenInfoResponse {
    /// Should be a url point to a json file
    pub url: String,
}

#[cw_serde]
pub struct TokensResponse {
    /// Contains all token_ids in lexicographical ordering
    /// If there are more than `limit`, use `start_from` in future queries
    /// to achieve pagination.
    pub tokens: Vec<TokenId>,
}
