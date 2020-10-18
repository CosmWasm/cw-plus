#![allow(dead_code)]

use cosmwasm_std::{from_slice, Api, Env, Extern, HandleResponse, MessageInfo, Querier, Storage};
use serde::de::DeserializeOwned;
use serde::export::PhantomData;
use std::collections::HashMap;

pub trait Handler<S, A, Q>
where
    S: Storage,
    A: Api,
    Q: Querier,
{
    fn handle(
        &self,
        deps: &mut Extern<S, A, Q>,
        env: Env,
        info: MessageInfo,
        msg: Vec<u8>,
    ) -> Result<HandleResponse, String>;
}

pub struct Contract<S, A, Q, T, E>
where
    S: Storage,
    A: Api,
    Q: Querier,
    T: DeserializeOwned,
    E: std::fmt::Display,
{
    handle_fn: fn(
        deps: &mut Extern<S, A, Q>,
        env: Env,
        info: MessageInfo,
        msg: T,
    ) -> Result<HandleResponse, E>,
    type_store: PhantomData<S>,
    type_api: PhantomData<A>,
    type_querier: PhantomData<Q>,
}

impl<S, A, Q, T, E> Handler<S, A, Q> for Contract<S, A, Q, T, E>
where
    S: Storage,
    A: Api,
    Q: Querier,
    T: DeserializeOwned,
    E: std::fmt::Display,
{
    fn handle(
        &self,
        deps: &mut Extern<S, A, Q>,
        env: Env,
        info: MessageInfo,
        msg: Vec<u8>,
    ) -> Result<HandleResponse, String> {
        let msg: T = from_slice(&msg).map_err(|e| e.to_string())?;
        let res = (self.handle_fn)(deps, env, info, msg);
        res.map_err(|e| e.to_string())
    }
}

pub struct Router<S, A, Q>
where
    S: Storage,
    A: Api,
    Q: Querier,
{
    handlers: HashMap<usize, Box<dyn Handler<S, A, Q>>>,
}

impl<S, A, Q> Router<S, A, Q>
where
    S: Storage,
    A: Api,
    Q: Querier,
{
    pub fn add_handler(&mut self, handler: Box<dyn Handler<S, A, Q>>) {
        let idx = self.handlers.len() + 1;
        self.handlers.insert(idx, handler);
    }

    // TODO: deps, env from inside router
    fn handle(
        &self,
        code_id: usize,
        deps: &mut Extern<S, A, Q>,
        env: Env,
        info: MessageInfo,
        msg: Vec<u8>,
    ) -> Result<HandleResponse, String> {
        let handler = self
            .handlers
            .get(&code_id)
            .ok_or_else(|| "Unregistered code id".to_string())?;
        handler.handle(deps, env, info, msg)
    }
}
