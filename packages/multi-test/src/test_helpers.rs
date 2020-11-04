#![cfg(test)]
use serde::{Deserialize, Serialize};

use crate::wasm::{Contract, ContractWrapper};
use cosmwasm_std::{
    Binary, Deps, DepsMut, Env, HandleResponse, InitResponse, MessageInfo, StdError,
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
