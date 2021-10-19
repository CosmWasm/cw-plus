// This whole file except `AdminListResponse` shall be generated form contract traits and
// instantiate signature

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{
    to_binary, Binary, CosmosMsg, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdError,
};

use crate::interfaces::Cw1Whitelist;

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct InstantiateMsg {
    pub admins: Vec<String>,
    pub mutable: bool,
}

// This one should be split to two interfaces:
// 1. Cw1ExeuteMsg for `Execute` (which is the only cw1 specific message)
// 2. Cw1WhitelistExecuteMsg for all other messages (which are specific for this implementation)
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg<T = Empty> {
    /// Execute requests the contract to re-dispatch all these messages with the
    /// contract's address as sender. Every implementation has it's own logic to
    /// determine in
    Execute { msgs: Vec<CosmosMsg<T>> },
    /// Freeze will make a mutable contract immutable, must be called by an admin
    Freeze {},
    /// UpdateAdmins will change the admin set of the contract, must be called by an existing admin,
    /// and only works if the contract is mutable
    UpdateAdmins { admins: Vec<String> },
}

impl<T> ExecuteMsg<T> {
    pub fn dispatch<Contract>(
        self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        contract: &Contract,
    ) -> Result<Response<T>, Contract::Error>
    where
        Contract: Cw1Whitelist<T>,
    {
        use ExecuteMsg::*;

        match self {
            Execute { msgs } => contract.execute(deps, env, info, msgs),
            Freeze {} => contract.freeze(deps, env, info),
            UpdateAdmins { admins } => contract.update_admins(deps, env, info, admins),
        }
    }
}

// This one should be split to two interfaces:
// 1. Cw1QueryMsg for `CanExecute` (which is the only cw1 specific message)
// 2. Cw1WhitelistQueryMsg for all `AdminList` (which are specific for this implementation)
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg<T = Empty> {
    /// Shows all admins and whether or not it is mutable
    AdminList {},
    /// Checks permissions of the caller on this proxy.
    /// If CanExecute returns true then a call to `Execute` with the same message,
    /// before any further state changes, should also succeed.
    CanExecute { sender: String, msg: CosmosMsg<T> },
}

impl<T> QueryMsg<T> {
    pub fn dispatch<Contract>(
        self,
        deps: Deps,
        env: Env,
        contract: &Contract,
    ) -> Result<Binary, Contract::Error>
    where
        Contract: Cw1Whitelist<T>,
        Contract::Error: From<StdError>,
    {
        use QueryMsg::*;

        match self {
            AdminList {} => to_binary(&contract.admin_list(deps, env)?),
            CanExecute { sender, msg } => to_binary(&contract.can_execute(deps, env, sender, msg)?),
        }
        .map_err(Contract::Error::from)
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct AdminListResponse {
    pub admins: Vec<String>,
    pub mutable: bool,
}
