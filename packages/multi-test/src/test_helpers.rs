#![cfg(test)]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;

use cosmwasm_std::{Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdError};
use cw_storage_plus::Item;

use crate::contracts::{Contract, ContractWrapper};

pub mod contracts;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EmptyMsg {}

/// This is just a demo place so we can test custom message handling
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename = "snake_case")]
pub enum CustomMsg {
    SetName { name: String },
    SetAge { age: u32 },
}

fn instantiate_error(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: EmptyMsg,
) -> Result<Response, StdError> {
    Err(StdError::generic_err("Init failed"))
}

fn execute_error(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: EmptyMsg,
) -> Result<Response, StdError> {
    Err(StdError::generic_err("Handle failed"))
}

fn query_error(_deps: Deps, _env: Env, _msg: EmptyMsg) -> Result<Binary, StdError> {
    Err(StdError::generic_err("Query failed"))
}

pub fn contract_error() -> Box<dyn Contract<Empty>> {
    let contract: ContractWrapper<_, _, _, _, _, _, _, _, _> =
        ContractWrapper::new(execute_error, instantiate_error, query_error);
    Box::new(contract)
}

#[allow(dead_code)]
pub fn contract_error_custom<C>() -> Box<dyn Contract<C>>
where
    C: Clone + fmt::Debug + PartialEq + JsonSchema + 'static,
{
    let contract: ContractWrapper<_, _, _, _, _, _, _, _, _> =
        ContractWrapper::new_with_empty(execute_error, instantiate_error, query_error);
    Box::new(contract)
}

const COUNT: Item<u32> = Item::new("count");
