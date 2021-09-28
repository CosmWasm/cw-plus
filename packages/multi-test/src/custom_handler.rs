use anyhow::{bail, Result as AnyResult};
use derivative::Derivative;
use std::cell::{Ref, RefCell};
use std::ops::Deref;
use std::rc::Rc;

use cosmwasm_std::{Addr, Api, Binary, BlockInfo, Empty, Querier, Storage};

use crate::app::CosmosRouter;
use crate::{AppResponse, Module};

/// Internal state of `CachingCustomHandler` wrapping internal mutability so it is not exposed to
/// user. Those have to be shared internal state, as after mock is passed to app it is not
/// possible to access mock internals which are not exposed by API.
#[derive(Derivative)]
#[derivative(Default(bound = "", new = "true"), Clone(bound = ""))]
pub struct CachingCustomHandlerState<ExecC, QueryC> {
    execs: Rc<RefCell<Vec<ExecC>>>,
    queries: Rc<RefCell<Vec<QueryC>>>,
}

impl<ExecC, QueryC> CachingCustomHandlerState<ExecC, QueryC> {
    pub fn execs(&self) -> impl Deref<Target = [ExecC]> + '_ {
        Ref::map(self.execs.borrow(), Vec::as_slice)
    }

    pub fn queries(&self) -> impl Deref<Target = [QueryC]> + '_ {
        Ref::map(self.queries.borrow(), Vec::as_slice)
    }

    pub fn reset(&self) {
        self.execs.borrow_mut().clear();
        self.queries.borrow_mut().clear();
    }
}

/// Custom handler storing all the messages it received, so they can be later verified. State is
/// thin shared state, so it can be hold after mock is passed to App to read state.
#[derive(Clone, Derivative)]
#[derivative(Default(bound = "", new = "true"))]
pub struct CachingCustomHandler<ExecC, QueryC> {
    state: CachingCustomHandlerState<ExecC, QueryC>,
}

impl<ExecC, QueryC> CachingCustomHandler<ExecC, QueryC> {
    pub fn state(&self) -> CachingCustomHandlerState<ExecC, QueryC> {
        self.state.clone()
    }
}

impl<Exec, Query> Module for CachingCustomHandler<Exec, Query> {
    type ExecT = Exec;
    type QueryT = Query;
    type SudoT = Empty;

    // TODO: how to assert
    // where ExecC: Exec, QueryC: Query
    fn execute<ExecC, QueryC>(
        &self,
        _api: &dyn Api,
        _storage: &mut dyn Storage,
        _router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        _block: &BlockInfo,
        _sender: Addr,
        msg: Self::ExecT,
    ) -> AnyResult<AppResponse> {
        self.state.execs.borrow_mut().push(msg);
        Ok(AppResponse::default())
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
        self.state.queries.borrow_mut().push(request);
        Ok(Binary::default())
    }
}
