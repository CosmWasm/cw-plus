use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::Uint128;
use cw_storage_plus::Item;

/// Supply is dynamic and tracks the current supply of staked and ERC20 tokens.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Default)]
pub struct CurveState {
    /// reserve is how many native tokens exist bonded to the validator
    pub reserve: Uint128,
    /// supply is how many tokens this contract has issued
    pub supply: Uint128,

    // what is the reserve denom
    pub reserve_denom: String,
}

impl CurveState {
    pub fn new(reserve_denom: String) -> Self {
        CurveState {
            reserve: Uint128(0),
            supply: Uint128(0),
            reserve_denom,
        }
    }
}

pub const CURVE_STATE: Item<CurveState> = Item::new("total_supply");

// TODO: add curve functions
// TODO: make this customizable in handle/query call
