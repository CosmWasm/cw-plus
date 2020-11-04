use serde::de::DeserializeOwned;
use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};

use cosmwasm_std::{
    from_slice, Api, Binary, BlockInfo, ContractInfo, Deps, DepsMut, Env, HandleResponse,
    HumanAddr, InitResponse, MessageInfo, Querier, QuerierWrapper, Storage,
};

/// Interface to call into a Contract
pub trait Contract {
    fn handle(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: Vec<u8>,
    ) -> Result<HandleResponse, String>;

    fn init(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: Vec<u8>,
    ) -> Result<InitResponse, String>;

    fn query(&self, deps: Deps, env: Env, msg: Vec<u8>) -> Result<Binary, String>;
}

type ContractFn<T, R, E> = fn(deps: DepsMut, env: Env, info: MessageInfo, msg: T) -> Result<R, E>;

type QueryFn<T, E> = fn(deps: Deps, env: Env, msg: T) -> Result<Binary, E>;

/// Wraps the exported functions from a contract and provides the normalized format
/// TODO: Allow to customize return values (CustomMsg beyond Empty)
/// TODO: Allow different error types?
pub struct ContractWrapper<T1, T2, T3, E>
where
    T1: DeserializeOwned,
    T2: DeserializeOwned,
    T3: DeserializeOwned,
    E: std::fmt::Display,
{
    handle_fn: ContractFn<T1, HandleResponse, E>,
    init_fn: ContractFn<T2, InitResponse, E>,
    query_fn: QueryFn<T3, E>,
}

impl<T1, T2, T3, E> ContractWrapper<T1, T2, T3, E>
where
    T1: DeserializeOwned,
    T2: DeserializeOwned,
    T3: DeserializeOwned,
    E: std::fmt::Display,
{
    pub fn new(
        handle_fn: ContractFn<T1, HandleResponse, E>,
        init_fn: ContractFn<T2, InitResponse, E>,
        query_fn: QueryFn<T3, E>,
    ) -> Self {
        ContractWrapper {
            handle_fn,
            init_fn,
            query_fn,
        }
    }
}

impl<T1, T2, T3, E> Contract for ContractWrapper<T1, T2, T3, E>
where
    T1: DeserializeOwned,
    T2: DeserializeOwned,
    T3: DeserializeOwned,
    E: std::fmt::Display,
{
    fn handle(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: Vec<u8>,
    ) -> Result<HandleResponse, String> {
        let msg: T1 = from_slice(&msg).map_err(|e| e.to_string())?;
        let res = (self.handle_fn)(deps, env, info, msg);
        res.map_err(|e| e.to_string())
    }

    fn init(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: Vec<u8>,
    ) -> Result<InitResponse, String> {
        let msg: T2 = from_slice(&msg).map_err(|e| e.to_string())?;
        let res = (self.init_fn)(deps, env, info, msg);
        res.map_err(|e| e.to_string())
    }

    fn query(&self, deps: Deps, env: Env, msg: Vec<u8>) -> Result<Binary, String> {
        let msg: T3 = from_slice(&msg).map_err(|e| e.to_string())?;
        let res = (self.query_fn)(deps, env, msg);
        res.map_err(|e| e.to_string())
    }
}

struct ContractData<S: Storage + Default> {
    code_id: usize,
    storage: RefCell<S>,
}

impl<S: Storage + Default> ContractData<S> {
    fn new(code_id: usize) -> Self {
        ContractData {
            code_id,
            storage: RefCell::new(S::default()),
        }
    }
}

pub struct WasmRouter<S>
where
    S: Storage + Default,
{
    handlers: HashMap<usize, Box<dyn Contract>>,
    contracts: HashMap<HumanAddr, ContractData<S>>,
    block: BlockInfo,
    api: Box<dyn Api>,
}

impl<S> WasmRouter<S>
where
    S: Storage + Default,
{
    pub fn new(api: Box<dyn Api>, block: BlockInfo) -> Self {
        WasmRouter {
            handlers: HashMap::new(),
            contracts: HashMap::new(),
            block,
            api,
        }
    }

    pub fn set_block(&mut self, block: BlockInfo) {
        self.block = block;
    }

    // this let's use use "next block" steps that add eg. one height and 5 seconds
    pub fn update_block<F: Fn(&mut BlockInfo)>(&mut self, action: F) {
        action(&mut self.block);
    }

    pub fn add_handler(&mut self, handler: Box<dyn Contract>) {
        let idx = self.handlers.len() + 1;
        self.handlers.insert(idx, handler);
    }

    /// This just creates an address and empty storage instance, returning the new address
    /// You must call init after this to set up the contract properly.
    /// These are separated into two steps to have cleaner return values.
    pub fn register_contract(&mut self, code_id: usize) -> Result<HumanAddr, String> {
        if !self.handlers.contains_key(&code_id) {
            return Err("Cannot init contract with unregistered code id".to_string());
        }
        // TODO: better addr generation
        let addr = HumanAddr::from(self.contracts.len().to_string());
        let info = ContractData::new(code_id);
        self.contracts.insert(addr.clone(), info);
        Ok(addr)
    }

    pub fn handle(
        &self,
        address: HumanAddr,
        querier: &dyn Querier,
        info: MessageInfo,
        msg: Vec<u8>,
    ) -> Result<HandleResponse, String> {
        self.with_storage(querier, address, |handler, deps, env| {
            handler.handle(deps, env, info, msg)
        })
    }

    pub fn init(
        &self,
        address: HumanAddr,
        querier: &dyn Querier,
        info: MessageInfo,
        msg: Vec<u8>,
    ) -> Result<InitResponse, String> {
        self.with_storage(querier, address, |handler, deps, env| {
            handler.init(deps, env, info, msg)
        })
    }

    pub fn query(
        &self,
        address: HumanAddr,
        querier: &dyn Querier,
        msg: Vec<u8>,
    ) -> Result<Binary, String> {
        self.with_storage(querier, address, |handler, deps, env| {
            handler.query(deps.as_ref(), env, msg)
        })
    }

    pub fn query_raw(&self, address: HumanAddr, key: &[u8]) -> Result<Binary, String> {
        let contract = self
            .contracts
            .get(&address)
            .ok_or_else(|| "Unregistered contract address".to_string())?;
        let storage = contract
            .storage
            .try_borrow()
            .map_err(|e| format!("Immutable borrowing failed - re-entrancy?: {}", e))?;
        let data = storage.get(&key).unwrap_or_default();
        Ok(data.into())
    }

    fn get_env<T: Into<HumanAddr>>(&self, address: T) -> Env {
        Env {
            block: self.block.clone(),
            contract: ContractInfo {
                address: address.into(),
            },
        }
    }

    fn with_storage<F, T>(
        &self,
        querier: &dyn Querier,
        address: HumanAddr,
        action: F,
    ) -> Result<T, String>
    where
        F: FnOnce(&Box<dyn Contract>, DepsMut, Env) -> Result<T, String>,
    {
        let contract = self
            .contracts
            .get(&address)
            .ok_or_else(|| "Unregistered contract address".to_string())?;
        let handler = self
            .handlers
            .get(&contract.code_id)
            .ok_or_else(|| "Unregistered code id".to_string())?;
        let env = self.get_env(address);

        let mut storage = contract
            .storage
            .try_borrow_mut()
            .map_err(|e| format!("Double-borrowing mutable storage - re-entrancy?: {}", e))?;
        let deps = DepsMut {
            storage: storage.deref_mut(),
            api: self.api.deref(),
            querier: QuerierWrapper::new(querier),
        };
        action(handler, deps, env)
    }
}
