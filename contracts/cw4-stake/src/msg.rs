use cosmwasm_std::{Coin, HumanAddr, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cw0::Duration;
use cw20::Denom;
pub use cw_controllers::ClaimsResponse;

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct InitMsg {
    /// denom of the token to stake
    pub denom: Denom,
    pub tokens_per_weight: Uint128,
    pub min_bond: Uint128,
    pub unbonding_period: Duration,

    // admin can only add/remove hooks, not change other parameters
    pub admin: Option<HumanAddr>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    /// Bond will bond all staking tokens sent with the message and update membership weight
    Bond {},
    /// Unbond will start the unbonding process for the given number of tokens.
    /// The sender immediately loses weight from these tokens, and can claim them
    /// back to his wallet after `unbonding_period`
    Unbond { tokens: Uint128 },
    /// Claim is used to claim your native tokens that you previously "unbonded"
    /// after the contract-defined waiting period (eg. 1 week)
    Claim {},

    /// Change the admin
    UpdateAdmin { admin: Option<HumanAddr> },
    /// Add a new hook to be informed of all membership changes. Must be called by Admin
    AddHook { addr: HumanAddr },
    /// Remove a hook. Must be called by Admin
    RemoveHook { addr: HumanAddr },
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Claims shows the tokens in process of unbonding for this address
    Claims {
        address: HumanAddr,
    },
    // Show the number of tokens currently staked by this address.
    Staked {
        address: HumanAddr,
    },

    /// Return AdminResponse
    Admin {},
    /// Return TotalWeightResponse
    TotalWeight {},
    /// Returns MembersListResponse
    ListMembers {
        start_after: Option<HumanAddr>,
        limit: Option<u32>,
    },
    /// Returns MemberResponse
    Member {
        addr: HumanAddr,
        at_height: Option<u64>,
    },
    /// Shows all registered hooks. Returns HooksResponse.
    Hooks {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct StakedResponse {
    pub stake: Coin,
}
