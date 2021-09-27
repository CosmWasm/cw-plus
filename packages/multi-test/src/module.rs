use std::marker::PhantomData;

use anyhow::{bail, Result as AnyResult};
use cosmwasm_std::{Addr, Api, Binary, BlockInfo, CustomQuery, Querier, Storage};

use crate::app::CosmosRouter;
use crate::AppResponse;
use schemars::JsonSchema;
use serde::de::DeserializeOwned;

pub trait Module {
    type ExecT;
    type QueryT;
    type SudoT;

    /// execute runs any ExecT message, which can be called by any external actor
    /// or smart contract
    fn execute<ExecC, QueryC>(
        &self,
        api: &dyn Api,
        storage: &mut dyn Storage,
        router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        block: &BlockInfo,
        sender: Addr,
        msg: Self::ExecT,
    ) -> AnyResult<AppResponse>
    where
        ExecC: std::fmt::Debug + Clone + PartialEq + JsonSchema + DeserializeOwned + 'static,
        QueryC: CustomQuery + DeserializeOwned + 'static;

    /// sudo runs privileged actions, like minting tokens, or governance proposals.
    /// This allows modules to have full access to these privileged actions,
    /// that cannot be triggered by smart contracts.
    ///
    /// There is no sender, as this must be previously authorized before the call
    fn sudo<ExecC, QueryC>(
        &self,
        api: &dyn Api,
        storage: &mut dyn Storage,
        router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        block: &BlockInfo,
        msg: Self::SudoT,
    ) -> AnyResult<AppResponse>
    where
        ExecC: std::fmt::Debug + Clone + PartialEq + JsonSchema + DeserializeOwned + 'static,
        QueryC: CustomQuery + DeserializeOwned + 'static;

    fn query(
        &self,
        api: &dyn Api,
        storage: &dyn Storage,
        querier: &dyn Querier,
        block: &BlockInfo,
        request: Self::QueryT,
    ) -> AnyResult<Binary>;
}

pub struct FailingModule<ExecT, QueryT, SudoT>(PhantomData<(ExecT, QueryT, SudoT)>);

impl<Exec, Query, Sudo> FailingModule<Exec, Query, Sudo> {
    pub fn new() -> Self {
        FailingModule(PhantomData)
    }
}

impl<Exec, Query, Sudo> Default for FailingModule<Exec, Query, Sudo> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Exec, Query, Sudo> Module for FailingModule<Exec, Query, Sudo>
where
    Exec: std::fmt::Debug,
    Query: std::fmt::Debug,
    Sudo: std::fmt::Debug,
{
    type ExecT = Exec;
    type QueryT = Query;
    type SudoT = Sudo;

    fn execute<ExecC, QueryC>(
        &self,
        _api: &dyn Api,
        _storage: &mut dyn Storage,
        _router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        _block: &BlockInfo,
        sender: Addr,
        msg: Self::ExecT,
    ) -> AnyResult<AppResponse> {
        bail!("Unexpected exec msg {:?} from {:?}", msg, sender)
    }

    fn sudo<ExecC, QueryC>(
        &self,
        _api: &dyn Api,
        _storage: &mut dyn Storage,
        _router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        _block: &BlockInfo,
        msg: Self::SudoT,
    ) -> AnyResult<AppResponse> {
        bail!("Unexpected sudo msg {:?}", msg)
    }

    fn query(
        &self,
        _api: &dyn Api,
        _storage: &dyn Storage,
        _querier: &dyn Querier,
        _block: &BlockInfo,
        request: Self::QueryT,
    ) -> AnyResult<Binary> {
        bail!("Unexpected custom query {:?}", request)
    }
}
