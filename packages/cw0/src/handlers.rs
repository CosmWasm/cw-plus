#![allow(dead_code)]
use serde::de::DeserializeOwned;
use std::cell::RefCell;
use std::collections::HashMap;

use crate::new_std::{ExternMut, ExternRef};
use cosmwasm_std::{
    from_slice, Api, Attribute, BankMsg, Binary, BlockInfo, Coin, ContractInfo, ContractResult,
    CosmosMsg, Empty, Env, HandleResponse, HumanAddr, InitResponse, MessageInfo, Querier,
    QuerierResult, QueryRequest, Storage, SystemError, SystemResult, WasmMsg, WasmQuery,
};
use std::ops::DerefMut;

/// Interface to call into a Contract
pub trait Contract<S, A, Q>
where
    S: Storage,
    A: Api,
    Q: Querier,
{
    fn handle(
        &self,
        deps: ExternMut<S, A, Q>,
        env: Env,
        info: MessageInfo,
        msg: Vec<u8>,
    ) -> Result<HandleResponse, String>;

    fn init(
        &self,
        deps: ExternMut<S, A, Q>,
        env: Env,
        info: MessageInfo,
        msg: Vec<u8>,
    ) -> Result<InitResponse, String>;

    fn query(&self, deps: ExternRef<S, A, Q>, env: Env, msg: Vec<u8>) -> Result<Binary, String>;
}

type ContractFn<S, A, Q, T, R, E> =
    fn(deps: ExternMut<S, A, Q>, env: Env, info: MessageInfo, msg: T) -> Result<R, E>;

type QueryFn<S, A, Q, T, E> = fn(deps: ExternRef<S, A, Q>, env: Env, msg: T) -> Result<Binary, E>;

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
        deps: ExternMut<S, A, Q>,
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
        deps: ExternMut<S, A, Q>,
        env: Env,
        info: MessageInfo,
        msg: Vec<u8>,
    ) -> Result<InitResponse, String> {
        let msg: T2 = from_slice(&msg).map_err(|e| e.to_string())?;
        let res = (self.init_fn)(deps, env, info, msg);
        res.map_err(|e| e.to_string())
    }

    fn query(&self, deps: ExternRef<S, A, Q>, env: Env, msg: Vec<u8>) -> Result<Binary, String> {
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
        querier: &Q,
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
        querier: &Q,
        info: MessageInfo,
        msg: Vec<u8>,
    ) -> Result<InitResponse, String> {
        self.with_storage(querier, address, |handler, deps, env| {
            handler.init(deps, env, info, msg)
        })
    }

    pub fn query(&self, address: HumanAddr, querier: &Q, msg: Vec<u8>) -> Result<Binary, String> {
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
        let data = storage.get(&key).unwrap_or(vec![]);
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

    fn with_storage<F, T>(&self, querier: &Q, address: HumanAddr, action: F) -> Result<T, String>
    where
        F: FnOnce(&Box<dyn Contract<S, A, Q>>, ExternMut<S, A, Q>, Env) -> Result<T, String>,
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
        let deps = ExternMut {
            storage: storage.deref_mut(),
            api: &self.api,
            querier,
        };
        let res = action(handler, deps, env);
        res
    }
}

#[derive(Default, Clone)]
pub struct RouterResponse {
    pub attributes: Vec<Attribute>,
    pub data: Option<Binary>,
}

// This can be InitResponse, HandleResponse, MigrationResponse
#[derive(Default, Clone)]
pub struct ActionResponse {
    // TODO: allow T != Empty
    pub messages: Vec<CosmosMsg<Empty>>,
    pub attributes: Vec<Attribute>,
    pub data: Option<Binary>,
}

impl From<HandleResponse<Empty>> for ActionResponse {
    fn from(input: HandleResponse<Empty>) -> Self {
        ActionResponse {
            messages: input.messages,
            attributes: input.attributes,
            data: input.data,
        }
    }
}

impl ActionResponse {
    fn init(input: InitResponse<Empty>, address: HumanAddr) -> Self {
        ActionResponse {
            messages: input.messages,
            attributes: input.attributes,
            data: Some(address.as_bytes().into()),
        }
    }
}

pub struct Router<S, A>
where
    S: Storage + Default,
    A: Api,
{
    wasm: WasmRouter<S, A, Router<S, A>>,
    // TODO: bank router
    // LATER: staking router
}

impl<S, A> Querier for Router<S, A>
where
    S: Storage + Default,
    A: Api,
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

impl<S, A> Router<S, A>
where
    S: Storage + Default,
    A: Api,
{
    // TODO: store BlockInfo in Router to change easier?
    pub fn new(api: A, block: BlockInfo) -> Self {
        Router {
            wasm: WasmRouter::new(api, block),
        }
    }

    pub fn execute(
        &mut self,
        sender: HumanAddr,
        msg: CosmosMsg<Empty>,
    ) -> Result<RouterResponse, String> {
        // TODO: we need to do some caching of storage here, once in the entry point
        // meaning, wrap current state.. all writes go to a cache... only when execute
        // returns a success do we flush it (otherwise drop it)
        self._execute(&sender, msg)
    }

    pub fn _execute(
        &mut self,
        sender: &HumanAddr,
        msg: CosmosMsg<Empty>,
    ) -> Result<RouterResponse, String> {
        match msg {
            CosmosMsg::Wasm(msg) => {
                let res = self.handle_wasm(sender, msg)?;
                let mut attributes = res.attributes;
                // recurse in all messages
                for resend in res.messages {
                    let subres = self._execute(sender, resend)?;
                    // ignore the data now, just like in wasmd
                    // append the events
                    attributes.extend_from_slice(&subres.attributes);
                }
                Ok(RouterResponse {
                    attributes,
                    data: res.data,
                })
            }
            CosmosMsg::Bank(msg) => self.handle_bank(sender, msg),
            _ => unimplemented!(),
        }
    }

    fn send<T: Into<HumanAddr>, U: Into<HumanAddr>>(
        &self,
        sender: T,
        recipient: U,
        amount: &[Coin],
    ) -> Result<RouterResponse, String> {
        if !amount.is_empty() {
            let sender: HumanAddr = sender.into();
            self.handle_bank(
                &sender,
                BankMsg::Send {
                    from_address: sender.clone(),
                    to_address: recipient.into(),
                    amount: amount.to_vec(),
                },
            )?;
        }
        Ok(RouterResponse::default())
    }

    fn handle_wasm(&mut self, sender: &HumanAddr, msg: WasmMsg) -> Result<ActionResponse, String> {
        match msg {
            WasmMsg::Execute {
                contract_addr,
                msg,
                send,
            } => {
                // first move the cash
                self.send(sender, &contract_addr, &send)?;
                // then call the contract
                let info = MessageInfo {
                    sender: sender.clone(),
                    sent_funds: send,
                };
                let res = self.wasm.handle(contract_addr, self, info, msg.to_vec())?;
                Ok(res.into())
            }
            WasmMsg::Instantiate {
                code_id,
                msg,
                send,
                label: _,
            } => {
                // register the contract
                let contract_addr = self.wasm.register_contract(code_id as usize)?;
                // move the cash
                self.send(sender, &contract_addr, &send)?;
                // then call the contract
                let info = MessageInfo {
                    sender: sender.clone(),
                    sent_funds: send,
                };
                let res = self
                    .wasm
                    .init(contract_addr.clone(), self, info, msg.to_vec())?;
                Ok(ActionResponse::init(res, contract_addr))
            }
        }
    }

    // Returns empty router response, just here for the same function signatures
    pub fn handle_bank(&self, sender: &HumanAddr, msg: BankMsg) -> Result<RouterResponse, String> {
        unimplemented!()
    }

    pub fn query(&self, request: QueryRequest<Empty>) -> Result<Binary, String> {
        match request {
            QueryRequest::Wasm(req) => self.query_wasm(req),
            QueryRequest::Bank(_) => unimplemented!(),
            _ => unimplemented!(),
        }
    }

    fn query_wasm(&self, request: WasmQuery) -> Result<Binary, String> {
        match request {
            WasmQuery::Smart { contract_addr, msg } => {
                self.wasm.query(contract_addr, self, msg.into())
            }
            WasmQuery::Raw { contract_addr, key } => self.wasm.query_raw(contract_addr, &key),
        }
    }
}
