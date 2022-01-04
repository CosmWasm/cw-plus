use std::fmt;

use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError};
use schemars::JsonSchema;

use crate::{test_helpers::EmptyMsg, Contract, ContractWrapper};

fn instantiate_err(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: EmptyMsg,
) -> Result<Response, StdError> {
    Err(StdError::generic_err("Init failed"))
}

fn instantiate_ok(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: EmptyMsg,
) -> Result<Response, StdError> {
    Ok(Response::default())
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

pub fn contract<C>(instantiable: bool) -> Box<dyn Contract<C>>
where
    C: Clone + fmt::Debug + PartialEq + JsonSchema + 'static,
{
    let contract = if instantiable {
        ContractWrapper::new_with_empty(execute, instantiate_ok, query)
    } else {
        ContractWrapper::new_with_empty(execute, instantiate_err, query)
    };
    Box::new(contract)
}
