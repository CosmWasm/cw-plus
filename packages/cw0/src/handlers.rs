#![allow(dead_code)]
use serde::de::DeserializeOwned;
use std::cell::Cell;
use std::collections::HashMap;

use cosmwasm_std::{
    from_slice, Api, Attribute, Binary, BlockInfo, ContractInfo, ContractResult, CosmosMsg, Empty,
    Env, Extern, HandleResponse, HumanAddr, InitResponse, MessageInfo, Querier, QuerierResult,
    QueryRequest, Storage, SystemError, SystemResult, WasmMsg, WasmQuery,
};

/// Interface to call into a Contract
pub trait Contract<S, A, Q>
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

    fn init(
        &self,
        deps: &mut Extern<S, A, Q>,
        env: Env,
        info: MessageInfo,
        msg: Vec<u8>,
    ) -> Result<InitResponse, String>;

    fn query(&self, deps: &Extern<S, A, Q>, env: Env, msg: Vec<u8>) -> Result<Binary, String>;
}

type ContractFn<S, A, Q, T, R, E> =
    fn(deps: &mut Extern<S, A, Q>, env: Env, info: MessageInfo, msg: T) -> Result<R, E>;

type QueryFn<S, A, Q, T, E> = fn(deps: &Extern<S, A, Q>, env: Env, msg: T) -> Result<Binary, E>;

/// Wraps the exported functions from a contract and provides the normalized format
/// TODO: Allow to customize return values (CustomMsg beyond Empty)
/// TODO: Allow different error types?
pub struct ContractWrapper<S, A, Q, T1, T2, T3, E>
where
    S: Storage,
    A: Api,
    Q: Querier,
    T1: DeserializeOwned,
    T2: DeserializeOwned,
    T3: DeserializeOwned,
    E: std::fmt::Display,
{
    handle_fn: ContractFn<S, A, Q, T1, HandleResponse, E>,
    init_fn: ContractFn<S, A, Q, T2, InitResponse, E>,
    query_fn: QueryFn<S, A, Q, T3, E>,
}

impl<S, A, Q, T1, T2, T3, E> ContractWrapper<S, A, Q, T1, T2, T3, E>
where
    S: Storage,
    A: Api,
    Q: Querier,
    T1: DeserializeOwned,
    T2: DeserializeOwned,
    T3: DeserializeOwned,
    E: std::fmt::Display,
{
    pub fn new(
        handle_fn: ContractFn<S, A, Q, T1, HandleResponse, E>,
        init_fn: ContractFn<S, A, Q, T2, InitResponse, E>,
        query_fn: QueryFn<S, A, Q, T3, E>,
    ) -> Self {
        ContractWrapper {
            handle_fn,
            init_fn,
            query_fn,
        }
    }
}

impl<S, A, Q, T1, T2, T3, E> Contract<S, A, Q> for ContractWrapper<S, A, Q, T1, T2, T3, E>
where
    S: Storage,
    A: Api,
    Q: Querier,
    T1: DeserializeOwned,
    T2: DeserializeOwned,
    T3: DeserializeOwned,
    E: std::fmt::Display,
{
    fn handle(
        &self,
        deps: &mut Extern<S, A, Q>,
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
        deps: &mut Extern<S, A, Q>,
        env: Env,
        info: MessageInfo,
        msg: Vec<u8>,
    ) -> Result<InitResponse, String> {
        let msg: T2 = from_slice(&msg).map_err(|e| e.to_string())?;
        let res = (self.init_fn)(deps, env, info, msg);
        res.map_err(|e| e.to_string())
    }

    fn query(&self, deps: &Extern<S, A, Q>, env: Env, msg: Vec<u8>) -> Result<Binary, String> {
        let msg: T3 = from_slice(&msg).map_err(|e| e.to_string())?;
        let res = (self.query_fn)(deps, env, msg);
        res.map_err(|e| e.to_string())
    }
}

struct ContractData<S: Storage + Default> {
    code_id: usize,
    storage: Cell<S>,
}

impl<S: Storage + Default> ContractData<S> {
    fn new(code_id: usize) -> Self {
        ContractData {
            code_id,
            storage: Cell::new(S::default()),
        }
    }
}

pub struct WasmRouter<S, A, Q>
where
    S: Storage + Default,
    A: Api,
    Q: Querier,
{
    handlers: HashMap<usize, Box<dyn Contract<S, A, Q>>>,
    contracts: HashMap<HumanAddr, ContractData<S>>,
    block: BlockInfo,
    api: A,
}

impl<S, A, Q> WasmRouter<S, A, Q>
where
    S: Storage + Default,
    A: Api,
    Q: Querier,
{
    // TODO: mock helper for the test defaults
    pub fn new(api: A, block: BlockInfo) -> Self {
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

    pub fn add_handler(&mut self, handler: Box<dyn Contract<S, A, Q>>) {
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
        querier: Q,
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
        querier: Q,
        info: MessageInfo,
        msg: Vec<u8>,
    ) -> Result<InitResponse, String> {
        self.with_storage(querier, address, |handler, deps, env| {
            handler.init(deps, env, info, msg)
        })
    }

    pub fn query(&self, address: HumanAddr, querier: Q, msg: Vec<u8>) -> Result<Binary, String> {
        self.with_storage(querier, address, |handler, deps, env| {
            handler.query(deps, env, msg)
        })
    }

    fn get_env<T: Into<HumanAddr>>(&self, address: T) -> Env {
        Env {
            block: self.block.clone(),
            contract: ContractInfo {
                address: address.into(),
            },
        }
    }

    fn with_storage<F, T>(&self, querier: Q, address: HumanAddr, action: F) -> Result<T, String>
    where
        F: FnOnce(&Box<dyn Contract<S, A, Q>>, &mut Extern<S, A, Q>, Env) -> Result<T, String>,
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

        let storage = contract.storage.take();
        let mut deps = Extern {
            storage,
            api: self.api,
            querier,
        };
        let res = action(handler, &mut deps, env);
        contract.storage.replace(deps.storage);
        res
    }
}

pub struct RouterResponse {
    pub attributes: Vec<Attribute>,
    pub data: Option<Binary>,
}

pub struct Router<S, A, Q>
where
    S: Storage + Default,
    A: Api,
    Q: Querier,
{
    wasm: WasmRouter<S, A, Q>,
    // TODO: bank router
    // LATER: staking router
}

impl<S, A, Q> Querier for Router<S, A, Q>
where
    S: Storage + Default,
    A: Api,
    Q: Querier,
{
    fn raw_query(&self, bin_request: &[u8]) -> QuerierResult {
        let request: QueryRequest<Empty> = match from_slice(bin_request) {
            Ok(v) => v,
            Err(e) => {
                return SystemResult::Err(SystemError::InvalidRequest {
                    error: format!("Parsing query request: {}", e),
                    request: bin_request.into(),
                })
            }
        };
        let contract_result: ContractResult<Binary> = self.query(request).into();
        SystemResult::Ok(contract_result)
    }
}

impl<S, A, Q> Router<S, A, Q>
where
    S: Storage + Default,
    A: Api,
    Q: Querier,
{
    pub fn new(api: A, block: BlockInfo) -> Self {
        unimplemented!();
    }

    pub fn handle(&self, msg: CosmosMsg<Empty>) -> Result<RouterResponse, String> {
        match msg {
            CosmosMsg::Wasm(msg) => self.handle_wasm(msg),
            CosmosMsg::Bank(_) => unimplemented!(),
            _ => unimplemented!(),
        }
    }

    fn handle_wasm(&self, msg: WasmMsg) -> Result<RouterResponse, String> {
        unimplemented!();
    }

    pub fn query(&self, request: QueryRequest<Empty>) -> Result<Binary, String> {
        match request {
            QueryRequest::Wasm(req) => self.query_wasm(req),
            QueryRequest::Bank(_) => unimplemented!(),
            _ => unimplemented!(),
        }
    }

    fn query_wasm(&self, msg: WasmQuery) -> Result<Binary, String> {
        unimplemented!();
    }
}
