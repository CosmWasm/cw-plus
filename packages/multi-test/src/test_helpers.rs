#![cfg(test)]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;

use cosmwasm_std::{
    attr, to_binary, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Empty, Env, MessageInfo,
    Response, StdError,
};
use cw_storage_plus::Item;

use crate::wasm::{Any, Contract, ContractWrapper};

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
    let contract: ContractWrapper<_, _, _, _, _, _, _, Any, Any> =
        ContractWrapper::new(handle_error, init_error, query_error);
    Box::new(contract)
}

#[allow(dead_code)]
pub fn contract_error_custom<C>() -> Box<dyn Contract<C>>
where
    C: Clone + fmt::Debug + PartialEq + JsonSchema + 'static,
{
    let contract: ContractWrapper<_, _, _, _, _, _, _, Any, Any> =
        ContractWrapper::new_with_empty(handle_error, init_error, query_error);
    Box::new(contract)
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PayoutMessage {
    pub payout: Coin,
}
const PAYOUT: Item<PayoutMessage> = Item::new("payout");

fn init_payout(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: PayoutMessage,
) -> Result<Response, StdError> {
    PAYOUT.save(deps.storage, &msg)?;
    Ok(Response::default())
}

fn handle_payout(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    _msg: EmptyMsg,
) -> Result<Response, StdError> {
    // always try to payout what was set originally
    let payout = PAYOUT.load(deps.storage)?;
    let msg = BankMsg::Send {
        to_address: info.sender.into(),
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
    let payout = PAYOUT.load(deps.storage)?;
    to_binary(&payout)
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

/// This is just a demo place so we can test custom message handling
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename = "snake_case")]
pub enum CustomMsg {
    SetName { name: String },
    SetAge { age: u32 },
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReflectSudoMsg {
    pub set_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReflectMessage {
    pub messages: Vec<CosmosMsg<CustomMsg>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReflectResponse {
    pub count: u32,
}

const REFLECT: Item<u32> = Item::new("reflect");

fn init_reflect(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: EmptyMsg,
) -> Result<Response<CustomMsg>, StdError> {
    REFLECT.save(deps.storage, &1)?;
    Ok(Response::default())
}

fn handle_reflect(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: ReflectMessage,
) -> Result<Response<CustomMsg>, StdError> {
    REFLECT.update::<_, StdError>(deps.storage, |old| Ok(old + 1))?;

    let res = Response {
        submessages: vec![],
        messages: msg.messages,
        attributes: vec![],
        data: None,
    };
    Ok(res)
}

fn sudo_reflect(
    deps: DepsMut,
    _env: Env,
    msg: ReflectSudoMsg,
) -> Result<Response<CustomMsg>, StdError> {
    REFLECT.save(deps.storage, &msg.set_count)?;
    Ok(Response::default())
}

fn query_reflect(deps: Deps, _env: Env, _msg: EmptyMsg) -> Result<Binary, StdError> {
    let count = REFLECT.load(deps.storage)?;
    let res = ReflectResponse { count };
    to_binary(&res)
}

pub fn contract_reflect() -> Box<dyn Contract<CustomMsg>> {
    let contract =
        ContractWrapper::new_with_sudo(handle_reflect, init_reflect, query_reflect, sudo_reflect);
    Box::new(contract)
}
