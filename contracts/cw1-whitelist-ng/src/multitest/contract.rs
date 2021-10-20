use crate::query::BoundCw1WhitelistQuerier;
use crate::state::Cw1WhitelistContract;
use crate::{msg::*, query::Cw1WhitelistQuerier};
use anyhow::{bail, Result as AnyResult};
use cosmwasm_std::{
    from_slice, Addr, Binary, Coin, CosmosMsg, CustomQuery, DepsMut, Empty, Env, MessageInfo,
    QuerierWrapper, Reply, Response,
};
use cw_multi_test::{AppResponse, Contract, Executor};
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use serde::Serialize;

impl<T> Contract<T> for Cw1WhitelistContract
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
        let msg: InstantiateMsg = from_slice(&msg)?;
        let InstantiateMsg { admins, mutable } = msg;
        self.instantiate(deps, env, info, admins, mutable)
            .map_err(Into::into)
    }

    fn execute(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: Vec<u8>,
    ) -> AnyResult<Response<T>> {
        let msg: ExecuteMsg<T> = from_slice(&msg)?;
        msg.dispatch(deps, env, info, self).map_err(Into::into)
    }

    fn query(&self, deps: cosmwasm_std::Deps, env: Env, msg: Vec<u8>) -> AnyResult<Binary> {
        let msg: QueryMsg<T> = from_slice(&msg)?;
        msg.dispatch(deps, env, self).map_err(Into::into)
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

// Proxy for direct execution in multitest.
#[derive(Clone, PartialEq, Debug)]
pub struct Cw1WhitelistProxy(Addr);

impl Cw1WhitelistProxy {
    pub fn addr(&self) -> Addr {
        self.0.clone()
    }

    #[allow(clippy::too_many_arguments)]
    pub fn instantiate<C, App, Admin>(
        app: &mut App,
        code_id: u64,
        sender: &Addr,
        admins: Vec<String>,
        mutable: bool,
        send_funds: &[Coin],
        label: &str,
        admin: Admin,
    ) -> AnyResult<Self>
    where
        C: Clone + std::fmt::Debug + PartialEq + JsonSchema + 'static,
        App: Executor<C>,
        Admin: Into<Option<String>>,
    {
        let addr = app.instantiate_contract(
            code_id,
            sender.clone(),
            &InstantiateMsg { admins, mutable },
            send_funds,
            label,
            admin.into(),
        )?;

        Ok(Self(addr))
    }

    pub fn execute<C, App>(
        &self,
        app: &mut App,
        sender: &Addr,
        msgs: Vec<CosmosMsg<C>>,
        send_funds: &[Coin],
    ) -> AnyResult<AppResponse>
    where
        C: Clone + std::fmt::Debug + PartialEq + JsonSchema + 'static + Serialize,
        App: Executor<C>,
    {
        app.execute_contract(
            sender.clone(),
            self.0.clone(),
            &ExecuteMsg::<C>::Execute { msgs },
            send_funds,
        )
    }

    pub fn freeze<C, App>(
        &self,
        app: &mut App,
        sender: &Addr,
        send_funds: &[Coin],
    ) -> AnyResult<AppResponse>
    where
        C: Clone + std::fmt::Debug + PartialEq + JsonSchema + 'static + Serialize,
        App: Executor<C>,
    {
        app.execute_contract(
            sender.clone(),
            self.0.clone(),
            &ExecuteMsg::<C>::Freeze {},
            send_funds,
        )
    }

    pub fn update_admins<C, App>(
        &self,
        app: &mut App,
        sender: &Addr,
        admins: Vec<String>,
        send_funds: &[Coin],
    ) -> AnyResult<AppResponse>
    where
        C: Clone + std::fmt::Debug + PartialEq + JsonSchema + 'static,
        App: Executor<C>,
    {
        app.execute_contract(
            sender.clone(),
            self.0.clone(),
            &ExecuteMsg::<Empty>::UpdateAdmins { admins },
            send_funds,
        )
    }

    // cw1_whitelist prefixed, as there is possibility to actually have multiple interfaces
    // implemented by single contract, every with separated querier
    pub fn cw1_whitelist_querier<'q, C>(
        &self,
        querier: &'q QuerierWrapper<'q, C>,
    ) -> BoundCw1WhitelistQuerier<'_, 'q, C>
    where
        C: CustomQuery,
    {
        Cw1WhitelistQuerier::new(&self.0, querier)
    }
}
