use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;

use cosmwasm_std::{
    to_binary, BankMsg, Binary, Coin, Deps, DepsMut, Env, MessageInfo, Response, StdError,
};
use cw_storage_plus::Item;

use crate::contracts::{Contract, ContractWrapper};
use crate::test_helpers::{EmptyMsg, COUNT};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InstantiateMessage {
    pub payout: Coin,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SudoMsg {
    pub set_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueryMsg {
    Count {},
    Payout {},
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CountResponse {
    pub count: u32,
}

const PAYOUT: Item<InstantiateMessage> = Item::new("payout");

fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMessage,
) -> Result<Response, StdError> {
    PAYOUT.save(deps.storage, &msg)?;
    COUNT.save(deps.storage, &1)?;
    Ok(Response::default())
}

fn execute(
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
    };
    Ok(Response::new()
        .add_message(msg)
        .add_attribute("action", "payout"))
}

fn sudo(deps: DepsMut, _env: Env, msg: SudoMsg) -> Result<Response, StdError> {
    COUNT.save(deps.storage, &msg.set_count)?;
    Ok(Response::default())
}

fn query(deps: Deps, _env: Env, msg: QueryMsg) -> Result<Binary, StdError> {
    match msg {
        QueryMsg::Count {} => {
            let count = COUNT.load(deps.storage)?;
            let res = CountResponse { count };
            to_binary(&res)
        }
        QueryMsg::Payout {} => {
            let payout = PAYOUT.load(deps.storage)?;
            to_binary(&payout)
        }
    }
}

pub fn contract<C>() -> Box<dyn Contract<C>>
where
    C: Clone + fmt::Debug + PartialEq + JsonSchema + 'static,
{
    let contract =
        ContractWrapper::new_with_empty(execute, instantiate, query).with_sudo_empty(sudo);
    Box::new(contract)
}
