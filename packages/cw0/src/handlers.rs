#![allow(dead_code)]

use cosmwasm_std::testing::MockQuerier;
use cosmwasm_std::{
    from_slice, Api, BlockInfo, ContractInfo, Env, Extern, HandleResponse, HumanAddr, MessageInfo,
    Querier, Storage,
};
use serde::de::DeserializeOwned;
use serde::export::PhantomData;
use std::cell::Cell;
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

// TODO: use Q not MockQuerier??
// We define one exact type of querier in our code
pub struct Router<S, A>
where
    S: Storage + Default,
    A: Api,
{
    handlers: HashMap<usize, Box<dyn Handler<S, A, MockQuerier>>>,
    contracts: HashMap<HumanAddr, ContractData<S>>,
    block: BlockInfo,
    api: A,
}

impl<S, A> Router<S, A>
where
    S: Storage + Default,
    A: Api,
{
    // TODO: mock helper for the test defaults
    pub fn new(api: A, block: BlockInfo) -> Self {
        Router {
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

    pub fn add_handler(&mut self, handler: Box<dyn Handler<S, A, MockQuerier>>) {
        let idx = self.handlers.len() + 1;
        self.handlers.insert(idx, handler);
    }

    // TODO: also run init here, and take InitMsg
    pub fn init_contract(&mut self, code_id: usize) -> Result<HumanAddr, String> {
        if !self.handlers.contains_key(&code_id) {
            return Err("Cannot init contract with unregistered code id".to_string());
        }
        // TODO: better addr generation
        let addr = HumanAddr::from(self.contracts.len().to_string());
        let info = ContractData::new(code_id);
        self.contracts.insert(addr.clone(), info);
        Ok(addr)
    }

    // TODO: deps from inside router
    fn handle(
        &self,
        address: HumanAddr,
        info: MessageInfo,
        msg: Vec<u8>,
    ) -> Result<HandleResponse, String> {
        let contract = self
            .contracts
            .get(&address)
            .ok_or_else(|| "Unregistered contract address".to_string())?;
        let handler = self
            .handlers
            .get(&contract.code_id)
            .ok_or_else(|| "Unregistered code id".to_string())?;

        // TODO: better way to recover here
        let storage = contract.storage.take();
        let mut deps = Extern {
            storage,
            api: self.api,
            querier: MockQuerier::new(&[(&address, &[])]),
        };
        let env = Env {
            block: self.block.clone(),
            contract: ContractInfo { address },
        };
        let res = handler.handle(&mut deps, env, info, msg);
        contract.storage.replace(deps.storage);
        res
    }
}
