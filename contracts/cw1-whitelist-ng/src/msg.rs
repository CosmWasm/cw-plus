// This whole file except `AdminListResponse` shall be generated form contract traits and
// instantiate signature

use cosmwasm_std::{Binary, CustomMsg, Deps, DepsMut, Env, MessageInfo, Response};
use schemars::JsonSchema;
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize};
use serde_cw_value::Value;

pub use crate::contract::msg::InstantiateMsg;

use crate::error::ContractError;
pub use crate::interfaces::{cw1_msg, whitelist};
use crate::state::Cw1WhitelistContract;

#[derive(Serialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(untagged)]
pub enum ExecMsg<T>
where
    T: CustomMsg,
{
    Cw1(crate::interfaces::cw1_msg::ExecMsg<T>),
    Whitelist(crate::interfaces::whitelist::ExecMsg),
}

impl<T> ExecMsg<T>
where
    T: CustomMsg,
{
    pub fn dispatch(
        self,
        contract: &Cw1WhitelistContract<T>,
        ctx: (DepsMut, Env, MessageInfo),
    ) -> Result<Response<T>, ContractError> {
        use ExecMsg::*;

        match self {
            Cw1(msg) => msg.dispatch(contract, ctx),
            Whitelist(msg) => msg.dispatch(contract, ctx),
        }
    }
}

impl<'de, T> Deserialize<'de> for ExecMsg<T>
where
    T: CustomMsg + Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let val = Value::deserialize(deserializer)?;

        let cw1_err = match val.clone().deserialize_into() {
            Ok(msg) => return Ok(Self::Cw1(msg)),
            Err(err) => err,
        };

        let whitelist_err = match val.deserialize_into() {
            Ok(msg) => return Ok(Self::Whitelist(msg)),
            Err(err) => err,
        };

        Err(D::Error::custom(format!(
                    "Expected Cw1 or Whitelist message, but cannot deserialize to neither of those.\nAs Cw1: {}\nAs Whitelist: {}", cw1_err, whitelist_err
                    )))
    }
}

#[derive(Serialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(untagged)]
pub enum QueryMsg<T>
where
    T: CustomMsg,
{
    Cw1(crate::interfaces::cw1_msg::QueryMsg<T>),
    Whitelist(crate::interfaces::whitelist::QueryMsg),
}

impl<T> QueryMsg<T>
where
    T: CustomMsg,
{
    pub fn dispatch(
        self,
        contract: &Cw1WhitelistContract<T>,
        ctx: (Deps, Env),
    ) -> Result<Binary, ContractError> {
        use QueryMsg::*;

        match self {
            Cw1(msg) => msg.dispatch(contract, ctx),
            Whitelist(msg) => msg.dispatch(contract, ctx),
        }
    }
}

impl<'de, T> Deserialize<'de> for QueryMsg<T>
where
    T: CustomMsg + Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let val = Value::deserialize(deserializer)?;

        let cw1_err = match val.clone().deserialize_into() {
            Ok(msg) => return Ok(Self::Cw1(msg)),
            Err(err) => err,
        };

        let whitelist_err = match val.deserialize_into() {
            Ok(msg) => return Ok(Self::Whitelist(msg)),
            Err(err) => err,
        };

        Err(D::Error::custom(format!(
                    "Expected Cw1 or Whitelist message, but cannot deserialize to neither of those.\nAs Cw1: {}\nAs Whitelist: {}", cw1_err, whitelist_err
                    )))
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct AdminListResponse {
    pub admins: Vec<String>,
    pub mutable: bool,
}
