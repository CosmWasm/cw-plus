//! Very simple echoing contract which just returns incomming string if any, but performming subcall of
//! given message to test response.
//!
//! Additionally it bypass all events and attributes send to it

use cosmwasm_std::{
    to_binary, Attribute, Binary, ContractResult, Deps, DepsMut, Empty, Env, Event, MessageInfo,
    Reply, Response, StdError, SubMsg, SubMsgExecutionResponse,
};
use serde::{Deserialize, Serialize};

use crate::{test_helpers::EmptyMsg, Contract, ContractWrapper};
use schemars::JsonSchema;
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Message {
    pub data: Option<String>,
    pub sub_msg: Vec<SubMsg>,
    pub attributes: Vec<Attribute>,
    pub events: Vec<Event>,
}

#[allow(clippy::unnecessary_wraps)]
fn instantiate(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: EmptyMsg,
) -> Result<Response, StdError> {
    Ok(Response::default())
}

#[allow(clippy::unnecessary_wraps)]
fn execute(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: Message,
) -> Result<Response, StdError> {
    let mut resp = Response::new();

    if let Some(data) = msg.data {
        resp = resp.set_data(data.into_bytes());
    }

    Ok(resp
        .add_submessages(msg.sub_msg)
        .add_attributes(msg.attributes)
        .add_events(msg.events))
}

fn query(_deps: Deps, _env: Env, msg: EmptyMsg) -> Result<Binary, StdError> {
    to_binary(&msg)
}

#[allow(clippy::unnecessary_wraps)]
fn reply(_deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, StdError> {
    if let Reply {
        result:
            ContractResult::Ok(SubMsgExecutionResponse {
                data: Some(data), ..
            }),
        ..
    } = msg
    {
        Ok(Response::new().set_data(data))
    } else {
        Ok(Response::new())
    }
}

pub fn contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(execute, instantiate, query).with_reply(reply);
    Box::new(contract)
}

pub fn custom_contract<C>() -> Box<dyn Contract<C>>
where
    C: Clone + fmt::Debug + PartialEq + JsonSchema + 'static,
{
    let contract = ContractWrapper::new_with_empty(execute, instantiate, query);
    let contract = contract.with_reply_empty(reply);
    Box::new(contract)
}
