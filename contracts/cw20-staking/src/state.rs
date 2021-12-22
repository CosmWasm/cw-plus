use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Decimal, Uint128};
use cw_controllers::Claims;
use cw_storage_plus::Item;
use cw_utils::Duration;

pub const CLAIMS: Claims = Claims::new("claims");

/// Investment info is fixed at instantiation, and is used to control the function of the contract
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InvestmentInfo {
    /// Owner created the contract and takes a cut
    pub owner: Addr,
    /// This is the denomination we can stake (and only one we accept for payments)
    pub bond_denom: String,
    /// This is the unbonding period of the native staking module
    /// We need this to only allow claims to be redeemed after the money has arrived
    pub unbonding_period: Duration,
    /// This is how much the owner takes as a cut when someone unbonds
    pub exit_tax: Decimal,
    /// All tokens are bonded to this validator
    /// FIXME: address validation doesn't work for validator addresses
    pub validator: String,
    /// This is the minimum amount we will pull out to reinvest, as well as a minimum
    /// that can be unbonded (to avoid needless staking tx)
    pub min_withdrawal: Uint128,
}

/// Supply is dynamic and tracks the current supply of staked and ERC20 tokens.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Default)]
pub struct Supply {
    /// issued is how many derivative tokens this contract has issued
    pub issued: Uint128,
    /// bonded is how many native tokens exist bonded to the validator
    pub bonded: Uint128,
    /// claims is how many tokens need to be reserved paying back those who unbonded
    pub claims: Uint128,
}

pub const INVESTMENT: Item<InvestmentInfo> = Item::new("invest");
pub const TOTAL_SUPPLY: Item<Supply> = Item::new("total_supply");
