//! Simplified contract which when executed releases the funds to beneficiary

use cosmwasm_std::{
    to_binary, BankMsg, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdError,
};
use cw_storage_plus::Item;
use serde::{Deserialize, Serialize};

use crate::{test_helpers::EmptyMsg, Contract, ContractWrapper};
use schemars::JsonSchema;
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstantiateMsg {
    pub beneficiary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrateMsg {
    // just use some other string so we see there are other types
    pub new_guy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    // returns InstantiateMsg
    Beneficiary {},
}

const HACKATOM: Item<InstantiateMsg> = Item::new("hackatom");

fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
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

fn query(deps: Deps, _env: Env, msg: QueryMsg) -> Result<Binary, StdError> {
    match msg {
        QueryMsg::Beneficiary {} => {
            let res = HACKATOM.load(deps.storage)?;
            to_binary(&res)
        }
    }
}

fn migrate(deps: DepsMut, _env: Env, msg: MigrateMsg) -> Result<Response, StdError> {
    HACKATOM.update::<_, StdError>(deps.storage, |mut state| {
        state.beneficiary = msg.new_guy;
        Ok(state)
    })?;
    let resp = Response::new().add_attribute("migrate", "successful");
    Ok(resp)
}

pub fn contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(execute, instantiate, query).with_migrate(migrate);
    Box::new(contract)
}

#[allow(dead_code)]
pub fn custom_contract<C>() -> Box<dyn Contract<C>>
where
    C: Clone + fmt::Debug + PartialEq + JsonSchema + 'static,
{
    let contract =
        ContractWrapper::new_with_empty(execute, instantiate, query).with_migrate_empty(migrate);
    Box::new(contract)
}
