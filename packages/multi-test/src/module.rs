use std::marker::PhantomData;

use anyhow::Result as AnyResult;
use cosmwasm_std::{Addr, Api, Binary, BlockInfo, Querier, Storage};

use crate::app::CosmosRouter;
use crate::AppResponse;

pub trait Module<ExecT, QueryT> {
    fn execute<ExecC, QueryC>(
        &self,
        api: &dyn Api,
        storage: &mut dyn Storage,
        router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        block: &BlockInfo,
        sender: Addr,
        msg: ExecT,
    ) -> AnyResult<AppResponse>;

    fn query(
        &self,
        api: &dyn Api,
        storage: &dyn Storage,
        querier: &dyn Querier,
        block: &BlockInfo,
        request: QueryT,
    ) -> AnyResult<Binary>;
}

pub struct PanickingModule<ExecT, QueryT>(PhantomData<(ExecT, QueryT)>);

impl<Exec, Query> PanickingModule<Exec, Query> {
    pub fn new() -> Self {
        PanickingModule(PhantomData)
    }
}

impl<Exec, Query> Default for PanickingModule<Exec, Query> {
    fn default() -> Self {
        Self::new()
    }
}

impl<ExecT, QueryT> Module<ExecT, QueryT> for PanickingModule<ExecT, QueryT>
where
    ExecT: std::fmt::Debug,
    QueryT: std::fmt::Debug,
{
    fn execute<ExecC, QueryC>(
        &self,
        _api: &dyn Api,
        _storage: &mut dyn Storage,
        _router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        _block: &BlockInfo,
        sender: Addr,
        msg: ExecT,
    ) -> AnyResult<AppResponse> {
        panic!("Unexpected exec msg {:?} from {:?}", msg, sender)
    }

    fn query(
        &self,
        _api: &dyn Api,
        _storage: &dyn Storage,
        _querier: &dyn Querier,
        _block: &BlockInfo,
        request: QueryT,
    ) -> AnyResult<Binary> {
        panic!("Unexpected custom query {:?}", request)
    }
}
