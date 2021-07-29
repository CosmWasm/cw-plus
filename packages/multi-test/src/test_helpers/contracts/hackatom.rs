//! Simplified contract which when executed releases the funds to beneficiary

use cosmwasm_std::{
    to_binary, BankMsg, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdError,
};
use cw_storage_plus::Item;
use serde::{Deserialize, Serialize};

use crate::{test_helpers::EmptyMsg, Contract, ContractWrapper};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitMsg {
    pub beneficiary: String,
}

const HACKATOM: Item<InitMsg> = Item::new("hackatom");

fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InitMsg,
) -> Result<Response, StdError> {
    HACKATOM.save(deps.storage, &msg)?;
    Ok(Response::default())
}

fn execute(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    _msg: EmptyMsg,
) -> Result<Response, StdError> {
    let init = HACKATOM.load(deps.storage)?;
    let balance = deps.querier.query_all_balances(env.contract.address)?;

    let resp = Response::new().add_message(BankMsg::Send {
        to_address: init.beneficiary,
        amount: balance,
    });

    Ok(resp)
}

fn query(_deps: Deps, _env: Env, msg: EmptyMsg) -> Result<Binary, StdError> {
    to_binary(&msg)
}

pub fn contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(execute, instantiate, query);
    Box::new(contract)
}
