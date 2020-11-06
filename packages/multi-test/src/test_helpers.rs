#![cfg(test)]
use serde::{Deserialize, Serialize};

use crate::wasm::{Contract, ContractWrapper};
use cosmwasm_std::{
    attr, from_slice, to_binary, to_vec, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Empty,
    Env, HandleResponse, InitResponse, MessageInfo, StdError,
};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EmptyMsg {}

fn init_error(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: EmptyMsg,
) -> Result<InitResponse, StdError> {
    Err(StdError::generic_err("Init failed"))
}

fn handle_error(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: EmptyMsg,
) -> Result<HandleResponse, StdError> {
    Err(StdError::generic_err("Handle failed"))
}

fn query_error(_deps: Deps, _env: Env, _msg: EmptyMsg) -> Result<Binary, StdError> {
    Err(StdError::generic_err("Query failed"))
}

pub fn contract_error() -> Box<dyn Contract> {
    let contract = ContractWrapper::new(handle_error, init_error, query_error);
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
) -> Result<InitResponse, StdError> {
    let bin = to_vec(&msg)?;
    deps.storage.set(PAYOUT_KEY, &bin);
    Ok(InitResponse::default())
}

fn handle_payout(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    _msg: EmptyMsg,
) -> Result<HandleResponse, StdError> {
    // always try to payout what was set originally
    let bin = deps.storage.get(PAYOUT_KEY).unwrap();
    let payout: PayoutMessage = from_slice(&bin)?;
    let msg = BankMsg::Send {
        from_address: env.contract.address,
        to_address: info.sender,
        amount: vec![payout.payout],
    }
    .into();
    let res = HandleResponse {
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

pub fn contract_payout() -> Box<dyn Contract> {
    let contract = ContractWrapper::new(handle_payout, init_payout, query_payout);
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
) -> Result<InitResponse, StdError> {
    deps.storage.set(REFLECT_KEY, &[1]);
    Ok(InitResponse::default())
}

fn handle_reflect(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: ReflectMessage,
) -> Result<HandleResponse, StdError> {
    let old = match deps.storage.get(REFLECT_KEY) {
        Some(bz) => bz[0],
        None => 0,
    };
    deps.storage.set(REFLECT_KEY, &[old + 1]);

    let res = HandleResponse {
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

pub fn contract_reflect() -> Box<dyn Contract> {
    let contract = ContractWrapper::new(handle_reflect, init_reflect, query_reflect);
    Box::new(contract)
}
