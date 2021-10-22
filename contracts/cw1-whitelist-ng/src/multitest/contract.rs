use crate::msg::*;
use crate::query::*;
use crate::state::Cw1WhitelistContract;
use anyhow::{bail, Result as AnyResult};
use cosmwasm_std::{
    Addr, Binary, Coin, CosmosMsg, CustomQuery, DepsMut, Env, MessageInfo, QuerierWrapper, Reply,
    Response,
};
use cw_multi_test::{AppResponse, Contract, Executor};
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use serde::Serialize;

impl<T> Contract<T> for Cw1WhitelistContract<T>
where
    T: Clone + std::fmt::Debug + PartialEq + JsonSchema + DeserializeOwned,
{
    fn instantiate(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: Vec<u8>,
    ) -> AnyResult<Response<T>> {
        self.entry_instantiate(deps, env, info, &msg)
            .map_err(Into::into)
    }

    fn execute(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: Vec<u8>,
    ) -> AnyResult<Response<T>> {
        self.entry_execute(deps, env, info, &msg)
            .map_err(Into::into)
    }

    fn query(&self, deps: cosmwasm_std::Deps, env: Env, msg: Vec<u8>) -> AnyResult<Binary> {
        self.entry_query(deps, env, &msg).map_err(Into::into)
    }

    fn sudo(&self, _deps: DepsMut, _env: Env, _msg: Vec<u8>) -> AnyResult<Response<T>> {
        bail!("sudo not implemented for contract")
    }

    fn reply(&self, _deps: DepsMut, _env: Env, _msg: Reply) -> AnyResult<Response<T>> {
        bail!("reply not implemented for contract")
    }

    fn migrate(&self, _deps: DepsMut, _env: Env, _msg: Vec<u8>) -> AnyResult<Response<T>> {
        bail!("migrate not implemented for contract")
    }
}

#[derive(PartialEq, Debug)]
#[must_use]
pub struct Cw1Executor<'a, App> {
    addr: Addr,
    app: &'a mut App,
    sender: Addr,
    send_funds: &'a [Coin],
}

impl<'a, App> Cw1Executor<'a, App> {
    pub fn new(addr: Addr, app: &'a mut App, sender: Addr, send_funds: &'a [Coin]) -> Self {
        Self {
            addr,
            app,
            sender,
            send_funds,
        }
    }

    pub fn execute<C>(self, msgs: Vec<CosmosMsg<C>>) -> AnyResult<AppResponse>
    where
        C: Clone + std::fmt::Debug + PartialEq + JsonSchema + Serialize + 'static,
        App: Executor<C>,
    {
        self.app.execute_contract(
            self.sender,
            self.addr,
            &Cw1ExecMsg::Execute { msgs },
            self.send_funds,
        )
    }
}

#[derive(PartialEq, Debug)]
#[must_use]
pub struct WhitelistExecutor<'a, App> {
    addr: Addr,
    app: &'a mut App,
    sender: Addr,
    send_funds: &'a [Coin],
}

impl<'a, App> WhitelistExecutor<'a, App> {
    pub fn new(addr: Addr, app: &'a mut App, sender: Addr, send_funds: &'a [Coin]) -> Self {
        Self {
            addr,
            app,
            sender,
            send_funds,
        }
    }

    pub fn freeze<C>(self) -> AnyResult<AppResponse>
    where
        C: Clone + std::fmt::Debug + PartialEq + JsonSchema + Serialize + 'static,
        App: Executor<C>,
    {
        self.app.execute_contract(
            self.sender,
            self.addr,
            &WhitelistExecMsg::Freeze {},
            self.send_funds,
        )
    }

    pub fn update_admins<C>(self, admins: Vec<String>) -> AnyResult<AppResponse>
    where
        C: Clone + std::fmt::Debug + PartialEq + JsonSchema + 'static + Serialize,
        App: Executor<C>,
    {
        self.app.execute_contract(
            self.sender,
            self.addr,
            &WhitelistExecMsg::UpdateAdmins { admins },
            self.send_funds,
        )
    }
}

#[derive(PartialEq, Debug)]
#[must_use]
pub struct Instantiator<'a, App> {
    code_id: u64,
    app: &'a mut App,
    sender: Addr,
    send_funds: &'a [Coin],
    label: String,
    admin: Option<String>,
}

impl<'a, App> Instantiator<'a, App> {
    pub fn new(code_id: u64, app: &'a mut App, sender: Addr, send_funds: &'a [Coin]) -> Self {
        Self {
            code_id,
            app,
            sender,
            send_funds,
            label: "Cw1Whitelist".to_owned(),
            admin: None,
        }
    }

    pub fn with_label(mut self, label: &str) -> Self {
        self.label = label.to_owned();
        self
    }

    pub fn with_admin(mut self, admin: &str) -> Self {
        self.admin = Some(admin.to_owned());
        self
    }

    pub fn with_args<C>(self, admins: Vec<String>, mutable: bool) -> AnyResult<Cw1WhitelistProxy>
    where
        C: Clone + std::fmt::Debug + PartialEq + JsonSchema + 'static,
        App: Executor<C>,
    {
        let addr = self.app.instantiate_contract(
            self.code_id,
            self.sender,
            &InstantiateMsg { admins, mutable },
            self.send_funds,
            self.label,
            self.admin,
        )?;

        Ok(Cw1WhitelistProxy(addr))
    }
}

// Proxy for direct execution in multitest.
#[derive(Clone, PartialEq, Debug)]
pub struct Cw1WhitelistProxy(Addr);

impl Cw1WhitelistProxy {
    pub fn addr(&self) -> Addr {
        self.0.clone()
    }

    pub fn instantiate<'a, App>(
        app: &'a mut App,
        code_id: u64,
        sender: &Addr,
        send_funds: &'a [Coin],
    ) -> Instantiator<'a, App> {
        Instantiator::new(code_id, app, sender.clone(), send_funds)
    }

    pub fn cw1_exec<'a, App>(
        &self,
        app: &'a mut App,
        sender: &Addr,
        send_funds: &'a [Coin],
    ) -> Cw1Executor<'a, App> {
        Cw1Executor::new(self.0.clone(), app, sender.clone(), send_funds)
    }

    pub fn whitelist_exec<'a, App>(
        &self,
        app: &'a mut App,
        sender: &Addr,
        send_funds: &'a [Coin],
    ) -> WhitelistExecutor<'a, App> {
        WhitelistExecutor::new(self.0.clone(), app, sender.clone(), send_funds)
    }

    pub fn cw1_querier<'a, C>(&'a self, querier: &'a QuerierWrapper<'a, C>) -> Cw1Querier<'a, C>
    where
        C: CustomQuery,
    {
        Cw1Querier::new(&self.0, querier)
    }

    pub fn whitelist_querier<'a, C>(
        &'a self,
        querier: &'a QuerierWrapper<'a, C>,
    ) -> WhitelistQuerier<'a, C>
    where
        C: CustomQuery,
    {
        WhitelistQuerier::new(&self.0, querier)
    }
}
