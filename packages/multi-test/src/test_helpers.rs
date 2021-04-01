#![cfg(test)]
use serde::{Deserialize, Serialize};

use crate::wasm::{Contract, ContractWrapper};
use cosmwasm_std::{
    attr, from_slice, to_binary, to_vec, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Empty,
    Env, MessageInfo, Response, StdError,
};
use schemars::JsonSchema;
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EmptyMsg {}

fn init_error(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: EmptyMsg,
) -> Result<Response, StdError> {
    Err(StdError::generic_err("Init failed"))
}

fn handle_error(
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
    let contract: ContractWrapper<_, _, _, _, _, _, _, String, String> =
        ContractWrapper::new(handle_error, init_error, query_error);
    Box::new(contract)
}

pub fn contract_error_custom<C>() -> Box<dyn Contract<C>>
where
    C: Clone + fmt::Debug + PartialEq + JsonSchema + 'static,
{
    let contract: ContractWrapper<_, _, _, _, _, _, _, String, String> =
        ContractWrapper::new_with_empty(handle_error, init_error, query_error);
    Box::new(contract)
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PayoutMessage {
    pub payout: Coin,
}

const PAYOUT_KEY: &[u8] = b"payout";

fn init_payout(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: PayoutMessage,
) -> Result<Response, StdError> {
    let bin = to_vec(&msg)?;
    deps.storage.set(PAYOUT_KEY, &bin);
    Ok(Response::default())
}

fn handle_payout(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    _msg: EmptyMsg,
) -> Result<Response, StdError> {
    // always try to payout what was set originally
    let bin = deps.storage.get(PAYOUT_KEY).unwrap();
    let payout: PayoutMessage = from_slice(&bin)?;
    let msg = BankMsg::Send {
        to_address: info.sender,
        amount: vec![payout.payout],
    }
    .into();
    let res = Response {
        submessages: vec![],
        messages: vec![msg],
        attributes: vec![attr("action", "payout")],
        data: None,
    };
    Ok(res)
}

fn query_payout(deps: Deps, _env: Env, _msg: EmptyMsg) -> Result<Binary, StdError> {
    let bin = deps.storage.get(PAYOUT_KEY).unwrap();
    Ok(bin.into())
}

pub fn contract_payout() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(handle_payout, init_payout, query_payout);
    Box::new(contract)
}

pub fn contract_payout_custom<C>() -> Box<dyn Contract<C>>
where
    C: Clone + fmt::Debug + PartialEq + JsonSchema + 'static,
{
    let contract = ContractWrapper::new_with_empty(handle_payout, init_payout, query_payout);
    Box::new(contract)
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReflectMessage {
    pub messages: Vec<CosmosMsg<Empty>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReflectResponse {
    pub count: u8,
}

const REFLECT_KEY: &[u8] = b"reflect";

fn init_reflect(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: EmptyMsg,
) -> Result<Response, StdError> {
    deps.storage.set(REFLECT_KEY, &[1]);
    Ok(Response::default())
}

fn handle_reflect(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: ReflectMessage,
) -> Result<Response, StdError> {
    let old = match deps.storage.get(REFLECT_KEY) {
        Some(bz) => bz[0],
        None => 0,
    };
    deps.storage.set(REFLECT_KEY, &[old + 1]);

    let res = Response {
        submessages: vec![],
        messages: msg.messages,
        attributes: vec![],
        data: None,
    };
    Ok(res)
}

fn query_reflect(deps: Deps, _env: Env, _msg: EmptyMsg) -> Result<Binary, StdError> {
    let count = match deps.storage.get(REFLECT_KEY) {
        Some(bz) => bz[0],
        None => 0,
    };
    let res = ReflectResponse { count };
    to_binary(&res)
}

pub fn contract_reflect() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(handle_reflect, init_reflect, query_reflect);
    Box::new(contract)
}

pub fn contract_reflect_custom<C>() -> Box<dyn Contract<C>>
where
    C: Clone + fmt::Debug + PartialEq + JsonSchema + 'static,
{
    let contract = ContractWrapper::new_with_empty(handle_reflect, init_reflect, query_reflect);
    Box::new(contract)
}
