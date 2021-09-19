use std::marker::PhantomData;

use anyhow::{bail, Result as AnyResult};
use cosmwasm_std::{Addr, Api, Binary, BlockInfo, Querier, Storage};

use crate::app::CosmosRouter;
use crate::AppResponse;

pub trait Module {
    type ExecT;
    type QueryT;

    fn execute<ExecC, QueryC>(
        &self,
        api: &dyn Api,
        storage: &mut dyn Storage,
        router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        block: &BlockInfo,
        sender: Addr,
        msg: Self::ExecT,
    ) -> AnyResult<AppResponse>;

    fn query(
        &self,
        api: &dyn Api,
        storage: &dyn Storage,
        querier: &dyn Querier,
        block: &BlockInfo,
        request: Self::QueryT,
    ) -> AnyResult<Binary>;
}

pub struct FailingModule<ExecT, QueryT>(PhantomData<(ExecT, QueryT)>);

impl<Exec, Query> FailingModule<Exec, Query> {
    pub fn new() -> Self {
        FailingModule(PhantomData)
    }
}

impl<Exec, Query> Default for FailingModule<Exec, Query> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Exec, Query> Module for FailingModule<Exec, Query>
where
    Exec: std::fmt::Debug,
    Query: std::fmt::Debug,
{
    type ExecT = Exec;
    type QueryT = Query;

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
