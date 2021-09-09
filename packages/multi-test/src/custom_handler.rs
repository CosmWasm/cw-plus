use cosmwasm_std::{Addr, Api, Binary, BlockInfo, Storage};

use anyhow::Result as AnyResult;
use derivative::Derivative;
use std::cell::{Ref, RefCell};
use std::marker::PhantomData;
use std::ops::Deref;
use std::rc::Rc;

use crate::AppResponse;

/// Custom message handler trait. Implementor of this trait is mocking environment behavior on
/// given custom message.
pub trait CustomHandler {
    /// Custom exec message for this handler
    type ExecC;

    /// Custom query message for this handler
    type QueryC;

    fn execute(
        &self,
        api: &dyn Api,
        storage: &mut dyn Storage,
        block: &BlockInfo,
        sender: Addr,
        msg: Self::ExecC,
    ) -> AnyResult<AppResponse>;

    fn query(
        &self,
        api: &dyn Api,
        storage: &dyn Storage,
        block: &BlockInfo,
        msg: Self::QueryC,
    ) -> AnyResult<Binary>;
}

/// Custom handler implementation panicking on each call. Assuming, that unless specific behavior
/// is implemented, custom messages should not be send.
pub struct PanickingCustomHandler<ExecC, QueryC>(PhantomData<ExecC>, PhantomData<QueryC>);

impl<Exec, Query> PanickingCustomHandler<Exec, Query> {
    pub fn new() -> Self {
        PanickingCustomHandler(PhantomData, PhantomData)
    }
}

impl<Exec, Query> CustomHandler for PanickingCustomHandler<Exec, Query>
where
    Exec: std::fmt::Debug,
    Query: std::fmt::Debug,
{
    type ExecC = Exec;
    type QueryC = Query;

    fn execute(
        &self,
        _api: &dyn Api,
        _storage: &mut dyn Storage,
        _block: &BlockInfo,
        sender: Addr,
        msg: Self::ExecC,
    ) -> AnyResult<AppResponse> {
        panic!("Unexpected custom exec msg {:?} from {:?}", msg, sender)
    }

    fn query(
        &self,
        _api: &dyn Api,
        _storage: &dyn Storage,
        _block: &BlockInfo,
        msg: Self::QueryC,
    ) -> AnyResult<Binary> {
        panic!("Unexpected custom query {:?}", msg)
    }
}

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

impl<Exec, Query> CustomHandler for CachingCustomHandler<Exec, Query> {
    type ExecC = Exec;
    type QueryC = Query;

    fn execute(
        &self,
        _api: &dyn Api,
        _storage: &mut dyn Storage,
        _block: &BlockInfo,
        _sender: Addr,
        msg: Exec,
    ) -> AnyResult<AppResponse> {
        self.state.execs.borrow_mut().push(msg);
        Ok(AppResponse::default())
    }

    fn query(
        &self,
        _api: &dyn Api,
        _storage: &dyn Storage,
        _block: &BlockInfo,
        msg: Query,
    ) -> AnyResult<Binary> {
        self.state.queries.borrow_mut().push(msg);
        Ok(Binary::default())
    }
}
