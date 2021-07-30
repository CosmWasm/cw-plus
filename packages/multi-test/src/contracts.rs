use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use std::fmt;

use cosmwasm_std::{
    from_slice, Binary, CosmosMsg, Deps, DepsMut, Empty, Env, MessageInfo, Reply, Response, SubMsg,
};

/// Interface to call into a Contract
pub trait Contract<T>
where
    T: Clone + fmt::Debug + PartialEq + JsonSchema,
{
    fn execute(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: Vec<u8>,
    ) -> Result<Response<T>, String>;

    fn instantiate(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: Vec<u8>,
    ) -> Result<Response<T>, String>;

    fn query(&self, deps: Deps, env: Env, msg: Vec<u8>) -> Result<Binary, String>;

    fn sudo(&self, deps: DepsMut, env: Env, msg: Vec<u8>) -> Result<Response<T>, String>;

    fn reply(&self, deps: DepsMut, env: Env, msg: Reply) -> Result<Response<T>, String>;

    fn migrate(&self, deps: DepsMut, env: Env, msg: Vec<u8>) -> Result<Response<T>, String>;
}

type ContractFn<T, C, E> =
    fn(deps: DepsMut, env: Env, info: MessageInfo, msg: T) -> Result<Response<C>, E>;
type PermissionedFn<T, C, E> = fn(deps: DepsMut, env: Env, msg: T) -> Result<Response<C>, E>;
type ReplyFn<C, E> = fn(deps: DepsMut, env: Env, msg: Reply) -> Result<Response<C>, E>;
type QueryFn<T, E> = fn(deps: Deps, env: Env, msg: T) -> Result<Binary, E>;

type ContractClosure<T, C, E> = Box<dyn Fn(DepsMut, Env, MessageInfo, T) -> Result<Response<C>, E>>;
type PermissionedClosure<T, C, E> = Box<dyn Fn(DepsMut, Env, T) -> Result<Response<C>, E>>;
type ReplyClosure<C, E> = Box<dyn Fn(DepsMut, Env, Reply) -> Result<Response<C>, E>>;
type QueryClosure<T, E> = Box<dyn Fn(Deps, Env, T) -> Result<Binary, E>>;

/// Wraps the exported functions from a contract and provides the normalized format
/// Place T4 and E4 at the end, as we just want default placeholders for most contracts that don't have sudo
pub struct ContractWrapper<
    T1,
    T2,
    T3,
    E1,
    E2,
    E3,
    C = Empty,
    T4 = Empty,
    E4 = String,
    E5 = String,
    T6 = Empty,
    E6 = String,
> where
    T1: DeserializeOwned,
    T2: DeserializeOwned,
    T3: DeserializeOwned,
    T4: DeserializeOwned,
    T6: DeserializeOwned,
    E1: ToString,
    E2: ToString,
    E3: ToString,
    E4: ToString,
    E5: ToString,
    E6: ToString,
    C: Clone + fmt::Debug + PartialEq + JsonSchema,
{
    execute_fn: ContractClosure<T1, C, E1>,
    instantiate_fn: ContractClosure<T2, C, E2>,
    query_fn: QueryClosure<T3, E3>,
    sudo_fn: Option<PermissionedClosure<T4, C, E4>>,
    reply_fn: Option<ReplyClosure<C, E5>>,
    migrate_fn: Option<PermissionedClosure<T6, C, E6>>,
}

impl<T1, T2, T3, E1, E2, E3, C> ContractWrapper<T1, T2, T3, E1, E2, E3, C>
where
    T1: DeserializeOwned + 'static,
    T2: DeserializeOwned + 'static,
    T3: DeserializeOwned + 'static,
    E1: ToString + 'static,
    E2: ToString + 'static,
    E3: ToString + 'static,
    C: Clone + fmt::Debug + PartialEq + JsonSchema + 'static,
{
    pub fn new(
        execute_fn: ContractFn<T1, C, E1>,
        instantiate_fn: ContractFn<T2, C, E2>,
        query_fn: QueryFn<T3, E3>,
    ) -> Self {
        ContractWrapper {
            execute_fn: Box::new(execute_fn),
            instantiate_fn: Box::new(instantiate_fn),
            query_fn: Box::new(query_fn),
            sudo_fn: None,
            reply_fn: None,
            migrate_fn: None,
        }
    }

    /// this will take a contract that returns Response<Empty> and will "upgrade" it
    /// to Response<C> if needed to be compatible with a chain-specific extension
    pub fn new_with_empty(
        execute_fn: ContractFn<T1, Empty, E1>,
        instantiate_fn: ContractFn<T2, Empty, E2>,
        query_fn: QueryFn<T3, E3>,
    ) -> Self {
        ContractWrapper {
            execute_fn: customize_fn(execute_fn),
            instantiate_fn: customize_fn(instantiate_fn),
            query_fn: Box::new(query_fn),
            sudo_fn: None,
            reply_fn: None,
            migrate_fn: None,
        }
    }
}

impl<T1, T2, T3, E1, E2, E3, C, T4, E4, E5, T6, E6>
    ContractWrapper<T1, T2, T3, E1, E2, E3, C, T4, E4, E5, T6, E6>
where
    T1: DeserializeOwned + 'static,
    T2: DeserializeOwned + 'static,
    T3: DeserializeOwned + 'static,
    T4: DeserializeOwned + 'static,
    T6: DeserializeOwned + 'static,
    E1: ToString + 'static,
    E2: ToString + 'static,
    E3: ToString + 'static,
    E4: ToString + 'static,
    E5: ToString + 'static,
    E6: ToString + 'static,
    C: Clone + fmt::Debug + PartialEq + JsonSchema + 'static,
{
    pub fn with_sudo<T4A, E4A>(
        self,
        sudo_fn: PermissionedFn<T4A, C, E4A>,
    ) -> ContractWrapper<T1, T2, T3, E1, E2, E3, C, T4A, E4A, E5, T6, E6>
    where
        T4A: DeserializeOwned + 'static,
        E4A: ToString + 'static,
    {
        ContractWrapper {
            execute_fn: self.execute_fn,
            instantiate_fn: self.instantiate_fn,
            query_fn: self.query_fn,
            sudo_fn: Some(Box::new(sudo_fn)),
            reply_fn: self.reply_fn,
            migrate_fn: self.migrate_fn,
        }
    }

    pub fn with_reply<E5A>(
        self,
        reply_fn: ReplyFn<C, E5A>,
    ) -> ContractWrapper<T1, T2, T3, E1, E2, E3, C, T4, E4, E5A, T6, E6>
    where
        E5A: ToString + 'static,
    {
        ContractWrapper {
            execute_fn: self.execute_fn,
            instantiate_fn: self.instantiate_fn,
            query_fn: self.query_fn,
            sudo_fn: self.sudo_fn,
            reply_fn: Some(Box::new(reply_fn)),
            migrate_fn: self.migrate_fn,
        }
    }

    pub fn with_migrate<T6A, E6A>(
        self,
        migrate_fn: PermissionedFn<T6A, C, E6A>,
    ) -> ContractWrapper<T1, T2, T3, E1, E2, E3, C, T4, E4, E5, T6A, E6A>
    where
        T6A: DeserializeOwned + 'static,
        E6A: ToString + 'static,
    {
        ContractWrapper {
            execute_fn: self.execute_fn,
            instantiate_fn: self.instantiate_fn,
            query_fn: self.query_fn,
            sudo_fn: self.sudo_fn,
            reply_fn: self.reply_fn,
            migrate_fn: Some(Box::new(migrate_fn)),
        }
    }
}

fn customize_fn<T, C, E>(raw_fn: ContractFn<T, Empty, E>) -> ContractClosure<T, C, E>
where
    T: DeserializeOwned + 'static,
    E: ToString + 'static,
    C: Clone + fmt::Debug + PartialEq + JsonSchema + 'static,
{
    let customized =
        move |deps: DepsMut, env: Env, info: MessageInfo, msg: T| -> Result<Response<C>, E> {
            raw_fn(deps, env, info, msg).map(customize_response::<C>)
        };
    Box::new(customized)
}

fn customize_response<C>(resp: Response<Empty>) -> Response<C>
where
    C: Clone + fmt::Debug + PartialEq + JsonSchema,
{
    let mut customized_resp = Response::<C>::new()
        .add_submessages(resp.messages.into_iter().map(customize_msg::<C>))
        .add_events(resp.events)
        .add_attributes(resp.attributes);
    customized_resp.data = resp.data;
    customized_resp
}

fn customize_msg<C>(msg: SubMsg<Empty>) -> SubMsg<C>
where
    C: Clone + fmt::Debug + PartialEq + JsonSchema,
{
    SubMsg {
        msg: match msg.msg {
            CosmosMsg::Wasm(wasm) => CosmosMsg::Wasm(wasm),
            CosmosMsg::Bank(bank) => CosmosMsg::Bank(bank),
            CosmosMsg::Staking(staking) => CosmosMsg::Staking(staking),
            CosmosMsg::Custom(_) => unreachable!(),
            #[cfg(feature = "stargate")]
            CosmosMsg::Ibc(ibc) => CosmosMsg::Ibc(ibc),
            #[cfg(feature = "stargate")]
            CosmosMsg::Stargate { type_url, value } => CosmosMsg::Stargate { type_url, value },
            _ => panic!("unknown message variant {:?}", msg),
        },
        id: msg.id,
        gas_limit: msg.gas_limit,
        reply_on: msg.reply_on,
    }
}

impl<T1, T2, T3, E1, E2, E3, C, T4, E4, E5, T6, E6> Contract<C>
    for ContractWrapper<T1, T2, T3, E1, E2, E3, C, T4, E4, E5, T6, E6>
where
    T1: DeserializeOwned,
    T2: DeserializeOwned,
    T3: DeserializeOwned,
    T4: DeserializeOwned,
    T6: DeserializeOwned,
    E1: ToString,
    E2: ToString,
    E3: ToString,
    E4: ToString,
    E5: ToString,
    E6: ToString,
    C: Clone + fmt::Debug + PartialEq + JsonSchema,
{
    fn execute(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: Vec<u8>,
    ) -> Result<Response<C>, String> {
        let msg = from_slice(&msg).map_err(|e| e.to_string())?;
        let res = (self.execute_fn)(deps, env, info, msg);
        res.map_err(|e| e.to_string())
    }

    fn instantiate(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: Vec<u8>,
    ) -> Result<Response<C>, String> {
        let msg = from_slice(&msg).map_err(|e| e.to_string())?;
        let res = (self.instantiate_fn)(deps, env, info, msg);
        res.map_err(|e| e.to_string())
    }

    fn query(&self, deps: Deps, env: Env, msg: Vec<u8>) -> Result<Binary, String> {
        let msg = from_slice(&msg).map_err(|e| e.to_string())?;
        let res = (self.query_fn)(deps, env, msg);
        res.map_err(|e| e.to_string())
    }

    // this returns an error if the contract doesn't implement sudo
    fn sudo(&self, deps: DepsMut, env: Env, msg: Vec<u8>) -> Result<Response<C>, String> {
        let msg = from_slice(&msg).map_err(|e| e.to_string())?;
        let res = match &self.sudo_fn {
            Some(sudo) => sudo(deps, env, msg),
            None => return Err("sudo not implemented for contract".to_string()),
        };
        res.map_err(|e| e.to_string())
    }

    // this returns an error if the contract doesn't implement reply
    fn reply(&self, deps: DepsMut, env: Env, reply_data: Reply) -> Result<Response<C>, String> {
        let res = match &self.reply_fn {
            Some(reply) => reply(deps, env, reply_data),
            None => return Err("reply not implemented for contract".to_string()),
        };
        res.map_err(|e| e.to_string())
    }

    // this returns an error if the contract doesn't implement migrate
    fn migrate(&self, deps: DepsMut, env: Env, msg: Vec<u8>) -> Result<Response<C>, String> {
        let msg = from_slice(&msg).map_err(|e| e.to_string())?;
        let res = match &self.migrate_fn {
            Some(migrate) => migrate(deps, env, msg),
            None => return Err("migrate not implemented for contract".to_string()),
        };
        res.map_err(|e| e.to_string())
    }
}
