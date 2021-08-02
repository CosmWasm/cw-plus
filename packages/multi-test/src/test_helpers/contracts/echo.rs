//! Very simple echoing contract which just returns incomming string if any, but performming subcall of
//! given message to test response

use cosmwasm_std::{
    to_binary, Binary, ContractResult, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdError,
    SubMsg, SubMsgExecutionResponse,
};
use serde::{Deserialize, Serialize};

use crate::{test_helpers::EmptyMsg, Contract, ContractWrapper};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Message {
    pub data: Option<String>,
    pub sub_msg: Vec<SubMsg<Binary>>,
}

fn instantiate(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: EmptyMsg,
) -> Result<Response<Binary>, StdError> {
    Ok(Response::default())
}

fn execute(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: Message,
) -> Result<Response<Binary>, StdError> {
    let mut resp = Response::new();
    if let Some(data) = msg.data {
        resp = resp.set_data(data.into_bytes());
    }
    Ok(resp.add_submessages(msg.sub_msg))
}

fn query(_deps: Deps, _env: Env, msg: EmptyMsg) -> Result<Binary, StdError> {
    to_binary(&msg)
}

fn reply(_deps: DepsMut, _env: Env, msg: Reply) -> Result<Response<Binary>, StdError> {
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

pub fn contract() -> Box<dyn Contract<Binary>> {
    let contract = ContractWrapper::new(execute, instantiate, query).with_reply(reply);
    Box::new(contract)
}
