use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;

use cosmwasm_std::{BankMsg, DistributionMsg, Empty, StakingMsg, WasmMsg};

/// This is needed so we can embed CosmosMsg as a trait bound.
/// See https://github.com/CosmWasm/cosmwasm/pull/1098 for a proper solution
/// (when we can deprecate this one)
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CosmosMsg<T = Empty> {
    Bank(BankMsg),
    // by default we use RawMsg, but a contract can override that
    // to call into more app-specific code (whatever they define)
    Custom(T),
    #[cfg(feature = "staking")]
    Distribution(DistributionMsg),
    #[cfg(feature = "staking")]
    Staking(StakingMsg),
    Wasm(WasmMsg),
}

impl<T> From<CosmosMsg<T>> for cosmwasm_std::CosmosMsg<T>
where
    T: Clone + fmt::Debug + PartialEq + JsonSchema,
{
    fn from(input: CosmosMsg<T>) -> Self {
        match input {
            CosmosMsg::Bank(b) => cosmwasm_std::CosmosMsg::Bank(b),
            CosmosMsg::Custom(c) => cosmwasm_std::CosmosMsg::Custom(c),
            #[cfg(feature = "staking")]
            CosmosMsg::Distribution(d) => cosmwasm_std::CosmosMsg::Distribution(d),
            #[cfg(feature = "staking")]
            CosmosMsg::Staking(s) => cosmwasm_std::CosmosMsg::Staking(s),
            CosmosMsg::Wasm(w) => cosmwasm_std::CosmosMsg::Wasm(w),
        }
    }
}

impl<T> From<cosmwasm_std::CosmosMsg<T>> for CosmosMsg<T>
where
    T: Clone + fmt::Debug + PartialEq + JsonSchema,
{
    fn from(input: cosmwasm_std::CosmosMsg<T>) -> CosmosMsg<T> {
        match input {
            cosmwasm_std::CosmosMsg::Bank(b) => CosmosMsg::Bank(b),
            cosmwasm_std::CosmosMsg::Custom(c) => CosmosMsg::Custom(c),
            #[cfg(feature = "staking")]
            cosmwasm_std::CosmosMsg::Distribution(d) => CosmosMsg::Distribution(d),
            #[cfg(feature = "staking")]
            cosmwasm_std::CosmosMsg::Staking(s) => CosmosMsg::Staking(s),
            cosmwasm_std::CosmosMsg::Wasm(w) => CosmosMsg::Wasm(w),
            _ => panic!("Unsupported type"),
        }
    }
}

impl<T> From<BankMsg> for CosmosMsg<T>
where
    T: Clone + fmt::Debug + PartialEq + JsonSchema,
{
    fn from(msg: BankMsg) -> Self {
        CosmosMsg::Bank(msg)
    }
}

#[cfg(feature = "staking")]
impl<T> From<StakingMsg> for CosmosMsg<T>
where
    T: Clone + fmt::Debug + PartialEq + JsonSchema,
{
    fn from(msg: StakingMsg) -> Self {
        CosmosMsg::Staking(msg)
    }
}

#[cfg(feature = "staking")]
impl<T> From<DistributionMsg> for CosmosMsg<T>
where
    T: Clone + fmt::Debug + PartialEq + JsonSchema,
{
    fn from(msg: DistributionMsg) -> Self {
        CosmosMsg::Distribution(msg)
    }
}

impl<T> From<WasmMsg> for CosmosMsg<T>
where
    T: Clone + fmt::Debug + PartialEq + JsonSchema,
{
    fn from(msg: WasmMsg) -> Self {
        CosmosMsg::Wasm(msg)
    }
}
