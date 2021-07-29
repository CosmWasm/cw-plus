use std::fmt;

use cosmwasm_std::{Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdError};
use schemars::JsonSchema;

use crate::{test_helpers::EmptyMsg, Contract, ContractWrapper};

fn instantiate(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: EmptyMsg,
) -> Result<Response, StdError> {
    Err(StdError::generic_err("Init failed"))
}

fn execute(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: EmptyMsg,
) -> Result<Response, StdError> {
    Err(StdError::generic_err("Handle failed"))
}

fn query(_deps: Deps, _env: Env, _msg: EmptyMsg) -> Result<Binary, StdError> {
    Err(StdError::generic_err("Query failed"))
}

pub fn contract() -> Box<dyn Contract<Empty>> {
    let contract: ContractWrapper<_, _, _, _, _, _, _, _, _> =
        ContractWrapper::new(execute, instantiate, query);
    Box::new(contract)
}

#[allow(dead_code)]
pub fn contract_custom<C>() -> Box<dyn Contract<C>>
where
    C: Clone + fmt::Debug + PartialEq + JsonSchema + 'static,
{
    let contract: ContractWrapper<_, _, _, _, _, _, _, _, _> =
        ContractWrapper::new_with_empty(execute, instantiate, query);
    Box::new(contract)
}
