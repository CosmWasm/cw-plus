use cosmwasm_std::{from_slice, Api, Env, Extern, InitResponse, MessageInfo, Querier, Storage};
use serde::de::DeserializeOwned;
use serde::export::PhantomData;

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
    ) -> Result<InitResponse, String>;
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
    ) -> Result<InitResponse, E>,
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
    ) -> Result<InitResponse, String> {
        let msg: T = from_slice(&msg).map_err(|e| e.to_string())?;
        let res = (self.handle_fn)(deps, env, info, msg);
        res.map_err(|e| e.to_string())
    }
}
