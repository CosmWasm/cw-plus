use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use std::error::Error;
use std::fmt::{self, Debug, Display};

use cosmwasm_std::{
    from_slice, Binary, CosmosMsg, CustomQuery, Deps, DepsMut, Empty, Env, MessageInfo, Reply,
    Response, SubMsg,
};

use anyhow::{anyhow, bail, Result as AnyResult};

/// Interface to call into a Contract
pub trait Contract<T, Q = Empty>
where
    T: Clone + fmt::Debug + PartialEq + JsonSchema,
    Q: CustomQuery,
{
    fn execute(
        &self,
        deps: DepsMut<Q>,
        env: Env,
        info: MessageInfo,
        msg: Vec<u8>,
    ) -> AnyResult<Response<T>>;

    fn instantiate(
        &self,
        deps: DepsMut<Q>,
        env: Env,
        info: MessageInfo,
        msg: Vec<u8>,
    ) -> AnyResult<Response<T>>;

    fn query(&self, deps: Deps<Q>, env: Env, msg: Vec<u8>) -> AnyResult<Binary>;

    fn sudo(&self, deps: DepsMut<Q>, env: Env, msg: Vec<u8>) -> AnyResult<Response<T>>;

    fn reply(&self, deps: DepsMut<Q>, env: Env, msg: Reply) -> AnyResult<Response<T>>;

    fn migrate(&self, deps: DepsMut<Q>, env: Env, msg: Vec<u8>) -> AnyResult<Response<T>>;
}

type ContractFn<T, C, E, Q> =
    fn(deps: DepsMut<Q>, env: Env, info: MessageInfo, msg: T) -> Result<Response<C>, E>;
type PermissionedFn<T, C, E, Q> = fn(deps: DepsMut<Q>, env: Env, msg: T) -> Result<Response<C>, E>;
type ReplyFn<C, E, Q> = fn(deps: DepsMut<Q>, env: Env, msg: Reply) -> Result<Response<C>, E>;
type QueryFn<T, E, Q> = fn(deps: Deps<Q>, env: Env, msg: T) -> Result<Binary, E>;

type ContractClosure<T, C, E, Q> =
    Box<dyn Fn(DepsMut<Q>, Env, MessageInfo, T) -> Result<Response<C>, E>>;
type PermissionedClosure<T, C, E, Q> = Box<dyn Fn(DepsMut<Q>, Env, T) -> Result<Response<C>, E>>;
type ReplyClosure<C, E, Q> = Box<dyn Fn(DepsMut<Q>, Env, Reply) -> Result<Response<C>, E>>;
type QueryClosure<T, E, Q> = Box<dyn Fn(Deps<Q>, Env, T) -> Result<Binary, E>>;

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
    Q = Empty,
    T4 = Empty,
    E4 = anyhow::Error,
    E5 = anyhow::Error,
    T6 = Empty,
    E6 = anyhow::Error,
> where
    T1: DeserializeOwned + Debug,
    T2: DeserializeOwned,
    T3: DeserializeOwned,
    T4: DeserializeOwned,
    T6: DeserializeOwned,
    E1: Display + Debug + Send + Sync + 'static,
    E2: Display + Debug + Send + Sync + 'static,
    E3: Display + Debug + Send + Sync + 'static,
    E4: Display + Debug + Send + Sync + 'static,
    E5: Display + Debug + Send + Sync + 'static,
    E6: Display + Debug + Send + Sync + 'static,
    C: Clone + fmt::Debug + PartialEq + JsonSchema,
    Q: CustomQuery + DeserializeOwned + 'static,
{
    execute_fn: ContractClosure<T1, C, E1, Q>,
    instantiate_fn: ContractClosure<T2, C, E2, Q>,
    query_fn: QueryClosure<T3, E3, Q>,
    sudo_fn: Option<PermissionedClosure<T4, C, E4, Q>>,
    reply_fn: Option<ReplyClosure<C, E5, Q>>,
    migrate_fn: Option<PermissionedClosure<T6, C, E6, Q>>,
}

impl<T1, T2, T3, E1, E2, E3, C, Q> ContractWrapper<T1, T2, T3, E1, E2, E3, C, Q>
where
    T1: DeserializeOwned + Debug + 'static,
    T2: DeserializeOwned + 'static,
    T3: DeserializeOwned + 'static,
    E1: Display + Debug + Send + Sync + 'static,
    E2: Display + Debug + Send + Sync + 'static,
    E3: Display + Debug + Send + Sync + 'static,
    C: Clone + fmt::Debug + PartialEq + JsonSchema + 'static,
    Q: CustomQuery + DeserializeOwned + 'static,
{
    pub fn new(
        execute_fn: ContractFn<T1, C, E1, Q>,
        instantiate_fn: ContractFn<T2, C, E2, Q>,
        query_fn: QueryFn<T3, E3, Q>,
    ) -> Self {
        Self {
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
        execute_fn: ContractFn<T1, Empty, E1, Q>,
        instantiate_fn: ContractFn<T2, Empty, E2, Q>,
        query_fn: QueryFn<T3, E3, Q>,
    ) -> Self {
        Self {
            execute_fn: customize_fn(execute_fn),
            instantiate_fn: customize_fn(instantiate_fn),
            query_fn: Box::new(query_fn),
            sudo_fn: None,
            reply_fn: None,
            migrate_fn: None,
        }
    }
}

impl<T1, T2, T3, E1, E2, E3, C, Q, T4, E4, E5, T6, E6>
    ContractWrapper<T1, T2, T3, E1, E2, E3, C, Q, T4, E4, E5, T6, E6>
where
    T1: DeserializeOwned + Debug + 'static,
    T2: DeserializeOwned + 'static,
    T3: DeserializeOwned + 'static,
    T4: DeserializeOwned + 'static,
    T6: DeserializeOwned + 'static,
    E1: Display + Debug + Send + Sync + 'static,
    E2: Display + Debug + Send + Sync + 'static,
    E3: Display + Debug + Send + Sync + 'static,
    E4: Display + Debug + Send + Sync + 'static,
    E5: Display + Debug + Send + Sync + 'static,
    E6: Display + Debug + Send + Sync + 'static,
    C: Clone + fmt::Debug + PartialEq + JsonSchema + 'static,
    Q: CustomQuery + DeserializeOwned + 'static,
{
    pub fn with_sudo<T4A, E4A>(
        self,
        sudo_fn: PermissionedFn<T4A, C, E4A, Q>,
    ) -> ContractWrapper<T1, T2, T3, E1, E2, E3, C, Q, T4A, E4A, E5, T6, E6>
    where
        T4A: DeserializeOwned + 'static,
        E4A: Display + Debug + Send + Sync + 'static,
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

    pub fn with_sudo_empty<T4A, E4A>(
        self,
        sudo_fn: PermissionedFn<T4A, Empty, E4A, Q>,
    ) -> ContractWrapper<T1, T2, T3, E1, E2, E3, C, Q, T4A, E4A, E5, T6, E6>
    where
        T4A: DeserializeOwned + 'static,
        E4A: Display + Debug + Send + Sync + 'static,
    {
        ContractWrapper {
            execute_fn: self.execute_fn,
            instantiate_fn: self.instantiate_fn,
            query_fn: self.query_fn,
            sudo_fn: Some(customize_permissioned_fn(sudo_fn)),
            reply_fn: self.reply_fn,
            migrate_fn: self.migrate_fn,
        }
    }

    pub fn with_reply<E5A>(
        self,
        reply_fn: ReplyFn<C, E5A, Q>,
    ) -> ContractWrapper<T1, T2, T3, E1, E2, E3, C, Q, T4, E4, E5A, T6, E6>
    where
        E5A: Display + Debug + Send + Sync + 'static,
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

    /// A correlate of new_with_empty
    pub fn with_reply_empty<E5A>(
        self,
        reply_fn: ReplyFn<Empty, E5A, Q>,
    ) -> ContractWrapper<T1, T2, T3, E1, E2, E3, C, Q, T4, E4, E5A, T6, E6>
    where
        E5A: Display + Debug + Send + Sync + 'static,
    {
        ContractWrapper {
            execute_fn: self.execute_fn,
            instantiate_fn: self.instantiate_fn,
            query_fn: self.query_fn,
            sudo_fn: self.sudo_fn,
            reply_fn: Some(customize_permissioned_fn(reply_fn)),
            migrate_fn: self.migrate_fn,
        }
    }

    pub fn with_migrate<T6A, E6A>(
        self,
        migrate_fn: PermissionedFn<T6A, C, E6A, Q>,
    ) -> ContractWrapper<T1, T2, T3, E1, E2, E3, C, Q, T4, E4, E5, T6A, E6A>
    where
        T6A: DeserializeOwned + 'static,
        E6A: Display + Debug + Send + Sync + 'static,
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

    pub fn with_migrate_empty<T6A, E6A>(
        self,
        migrate_fn: PermissionedFn<T6A, Empty, E6A, Q>,
    ) -> ContractWrapper<T1, T2, T3, E1, E2, E3, C, Q, T4, E4, E5, T6A, E6A>
    where
        T6A: DeserializeOwned + 'static,
        E6A: Display + Debug + Send + Sync + 'static,
    {
        ContractWrapper {
            execute_fn: self.execute_fn,
            instantiate_fn: self.instantiate_fn,
            query_fn: self.query_fn,
            sudo_fn: self.sudo_fn,
            reply_fn: self.reply_fn,
            migrate_fn: Some(customize_permissioned_fn(migrate_fn)),
        }
    }
}

fn customize_fn<T, C, E, Q>(raw_fn: ContractFn<T, Empty, E, Q>) -> ContractClosure<T, C, E, Q>
where
    T: DeserializeOwned + 'static,
    E: Display + Debug + Send + Sync + 'static,
    C: Clone + fmt::Debug + PartialEq + JsonSchema + 'static,
    Q: CustomQuery + DeserializeOwned + 'static,
{
    let customized =
        move |deps: DepsMut<Q>, env: Env, info: MessageInfo, msg: T| -> Result<Response<C>, E> {
            raw_fn(deps, env, info, msg).map(customize_response::<C>)
        };
    Box::new(customized)
}

fn customize_permissioned_fn<T, C, E, Q>(
    raw_fn: PermissionedFn<T, Empty, E, Q>,
) -> PermissionedClosure<T, C, E, Q>
where
    T: DeserializeOwned + 'static,
    E: Display + Debug + Send + Sync + 'static,
    C: Clone + fmt::Debug + PartialEq + JsonSchema + 'static,
    Q: CustomQuery + DeserializeOwned + 'static,
{
    let customized = move |deps: DepsMut<Q>, env: Env, msg: T| -> Result<Response<C>, E> {
        raw_fn(deps, env, msg).map(customize_response::<C>)
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

impl<T1, T2, T3, E1, E2, E3, C, T4, E4, E5, T6, E6, Q> Contract<C, Q>
    for ContractWrapper<T1, T2, T3, E1, E2, E3, C, Q, T4, E4, E5, T6, E6>
where
    T1: DeserializeOwned + Debug + Clone,
    T2: DeserializeOwned + Debug + Clone,
    T3: DeserializeOwned + Debug + Clone,
    T4: DeserializeOwned,
    T6: DeserializeOwned,
    E1: Display + Debug + Send + Sync + Error + 'static,
    E2: Display + Debug + Send + Sync + Error + 'static,
    E3: Display + Debug + Send + Sync + Error + 'static,
    E4: Display + Debug + Send + Sync + 'static,
    E5: Display + Debug + Send + Sync + 'static,
    E6: Display + Debug + Send + Sync + 'static,
    C: Clone + fmt::Debug + PartialEq + JsonSchema,
    Q: CustomQuery + DeserializeOwned,
{
    fn execute(
        &self,
        deps: DepsMut<Q>,
        env: Env,
        info: MessageInfo,
        msg: Vec<u8>,
    ) -> AnyResult<Response<C>> {
        let msg: T1 = from_slice(&msg)?;
        (self.execute_fn)(deps, env, info, msg).map_err(|err| anyhow!(err))
    }

    fn instantiate(
        &self,
        deps: DepsMut<Q>,
        env: Env,
        info: MessageInfo,
        msg: Vec<u8>,
    ) -> AnyResult<Response<C>> {
        let msg: T2 = from_slice(&msg)?;
        (self.instantiate_fn)(deps, env, info, msg).map_err(|err| anyhow!(err))
    }

    fn query(&self, deps: Deps<Q>, env: Env, msg: Vec<u8>) -> AnyResult<Binary> {
        let msg: T3 = from_slice(&msg)?;
        (self.query_fn)(deps, env, msg).map_err(|err| anyhow!(err))
    }

    // this returns an error if the contract doesn't implement sudo
    fn sudo(&self, deps: DepsMut<Q>, env: Env, msg: Vec<u8>) -> AnyResult<Response<C>> {
        let msg = from_slice(&msg)?;
        match &self.sudo_fn {
            Some(sudo) => sudo(deps, env, msg).map_err(|err| anyhow!(err)),
            None => bail!("sudo not implemented for contract"),
        }
    }

    // this returns an error if the contract doesn't implement reply
    fn reply(&self, deps: DepsMut<Q>, env: Env, reply_data: Reply) -> AnyResult<Response<C>> {
        match &self.reply_fn {
            Some(reply) => reply(deps, env, reply_data).map_err(|err| anyhow!(err)),
            None => bail!("reply not implemented for contract"),
        }
    }

    // this returns an error if the contract doesn't implement migrate
    fn migrate(&self, deps: DepsMut<Q>, env: Env, msg: Vec<u8>) -> AnyResult<Response<C>> {
        let msg = from_slice(&msg)?;
        match &self.migrate_fn {
            Some(migrate) => migrate(deps, env, msg).map_err(|err| anyhow!(err)),
            None => bail!("migrate not implemented for contract"),
        }
    }
}
