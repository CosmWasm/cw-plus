use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;

use cosmwasm_std::{Coin, CosmosMsg, Empty, HumanAddr};
use cw0::Expiration;

use crate::balance::Balance;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg<T = Empty>
where
    T: Clone + fmt::Debug + PartialEq + JsonSchema,
{
    /// Execute requests the contract to re-dispatch all these messages with the
    /// contract's address as sender. Every implementation has it's own logic to
    /// determine in
    Execute { msgs: Vec<CosmosMsg<T>> },
    /// Freeze will make a mutable contract immutable, must be called by an admin
    Freeze {},
    /// UpdateAdmins will change the admin set of the contract, must be called by an existing admin,
    /// and only works if the contract is mutable
    UpdateAdmins { admins: Vec<HumanAddr> },

    /// Add an allowance to a given subkey (subkey must not be admin)
    IncreaseAllowance {
        spender: HumanAddr,
        amount: Coin,
        expires: Option<Expiration>,
    },
    /// Decreases an allowance for a given subkey (subkey must not be admin)
    DecreaseAllowance {
        spender: HumanAddr,
        amount: Coin,
        expires: Option<Expiration>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg<T = Empty>
where
    T: Clone + fmt::Debug + PartialEq + JsonSchema,
{
    /// Shows all admins and whether or not it is mutable
    /// Returns cw1-whitelist::AdminListResponse
    AdminList {},
    /// Get the current allowance for the given subkey (how much it can spend)
    /// Returns crate::state::Allowance
    Allowance { spender: HumanAddr },
    /// Checks permissions of the caller on this proxy.
    /// If CanSend returns true then a call to `Execute` with the same message,
    /// before any further state changes, should also succeed.
    CanSend {
        sender: HumanAddr,
        msg: CosmosMsg<T>,
    },
    /// Gets all Allowances for this contract
    /// Returns AllAllowancesResponse
    AllAllowances {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AllAllowancesResponse {
    pub allowances: Vec<AllowanceInfo>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AllowanceInfo {
    pub spender: HumanAddr,
    pub balance: Balance,
    pub expires: Expiration,
}
