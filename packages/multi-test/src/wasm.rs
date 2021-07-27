use std::collections::HashMap;
use std::fmt;
use std::ops::Deref;

use cosmwasm_std::{
    Addr, Api, BankMsg, Binary, BlockInfo, Coin, ContractInfo, ContractResult, Deps, DepsMut, Env,
    Event, MessageInfo, Order, Querier, QuerierWrapper, Reply, ReplyOn, Response, Storage, SubMsg,
    SubMsgExecutionResponse, WasmMsg, WasmQuery,
};
use cosmwasm_storage::{prefixed, prefixed_read, PrefixedStorage, ReadonlyPrefixedStorage};
use prost::Message;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cw_storage_plus::Map;

use crate::app::{Router, RouterQuerier};
use crate::contracts::Contract;
use crate::executor::AppResponse;
use crate::transactions::transactional;

// Contract state is kept in Storage, separate from the contracts themselves
const CONTRACTS: Map<&Addr, ContractData> = Map::new("contracts");

pub const NAMESPACE_WASM: &[u8] = b"wasm";

/// Contract Data is just a code_id that can be used to lookup the actual code from the Router
/// We can add other info here in the future, like admin
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
struct ContractData {
    pub code_id: usize,
}

impl ContractData {
    fn new(code_id: usize) -> Self {
        ContractData { code_id }
    }
}

pub trait Wasm<C>
where
    C: Clone + fmt::Debug + PartialEq + JsonSchema,
{
    /// Handles all WasmQuery requests
    fn query(
        &self,
        storage: &dyn Storage,
        querier: &dyn Querier,
        block: &BlockInfo,
        request: WasmQuery,
    ) -> Result<Binary, String>;

    /// Handles all WasmMsg messages
    fn execute(
        &self,
        storage: &mut dyn Storage,
        router: &Router<C>,
        block: &BlockInfo,
        sender: Addr,
        msg: WasmMsg,
    ) -> Result<AppResponse, String>;

    // Add a new contract. Must be done on the base object, when no contracts running
    fn store_code(&mut self, code: Box<dyn Contract<C>>) -> usize;

    /// Admin interface, cannot be called via CosmosMsg
    fn sudo(
        &self,
        contract_addr: Addr,
        storage: &mut dyn Storage,
        router: &Router<C>,
        block: &BlockInfo,
        msg: Vec<u8>,
    ) -> Result<AppResponse, String>;
}

pub struct WasmKeeper<C>
where
    C: Clone + fmt::Debug + PartialEq + JsonSchema,
{
    /// code is in-memory lookup that stands in for wasm code
    /// this can only be edited on the WasmRouter, and just read in caches
    codes: HashMap<usize, Box<dyn Contract<C>>>,

    // WasmConst
    api: Box<dyn Api>,
}

impl<C> Wasm<C> for WasmKeeper<C>
where
    C: Clone + fmt::Debug + PartialEq + JsonSchema,
{
    fn query(
        &self,
        storage: &dyn Storage,
        querier: &dyn Querier,
        block: &BlockInfo,
        request: WasmQuery,
    ) -> Result<Binary, String> {
        match request {
            WasmQuery::Smart { contract_addr, msg } => {
                let addr = self
                    .api
                    .addr_validate(&contract_addr)
                    .map_err(|e| e.to_string())?;
                self.query_smart(addr, storage, querier, block, msg.into())
            }
            WasmQuery::Raw { contract_addr, key } => {
                let addr = self
                    .api
                    .addr_validate(&contract_addr)
                    .map_err(|e| e.to_string())?;
                Ok(self.query_raw(addr, storage, &key))
            }
            q => panic!("Unsupported wasm query: {:?}", q),
        }
    }

    fn execute(
        &self,
        storage: &mut dyn Storage,
        router: &Router<C>,
        block: &BlockInfo,
        sender: Addr,
        msg: WasmMsg,
    ) -> Result<AppResponse, String> {
        let (resender, res) = self.execute_wasm(storage, router, block, sender, msg)?;
        self.process_response(router, storage, block, resender, res, false)
    }

    fn store_code(&mut self, code: Box<dyn Contract<C>>) -> usize {
        let idx = self.codes.len() + 1;
        self.codes.insert(idx, code);
        idx
    }

    fn sudo(
        &self,
        contract_addr: Addr,
        storage: &mut dyn Storage,
        router: &Router<C>,
        block: &BlockInfo,
        msg: Vec<u8>,
    ) -> Result<AppResponse, String> {
        let res = self.call_sudo(contract_addr.clone(), storage, router, block, msg)?;
        self.process_response(router, storage, block, contract_addr, res, false)
    }
}

impl<C> WasmKeeper<C>
where
    C: Clone + fmt::Debug + PartialEq + JsonSchema,
{
    pub fn new(api: Box<dyn Api>) -> Self {
        WasmKeeper {
            codes: HashMap::new(),
            api,
        }
    }

    pub fn query_smart(
        &self,
        address: Addr,
        storage: &dyn Storage,
        querier: &dyn Querier,
        block: &BlockInfo,
        msg: Vec<u8>,
    ) -> Result<Binary, String> {
        self.with_storage_readonly(storage, querier, block, address, |handler, deps, env| {
            handler.query(deps, env, msg)
        })
    }

    pub fn query_raw(&self, address: Addr, storage: &dyn Storage, key: &[u8]) -> Binary {
        let storage = self.contract_storage_readonly(storage, &address);
        let data = storage.get(&key).unwrap_or_default();
        data.into()
    }

    fn send<T: Into<Addr>>(
        &self,
        storage: &mut dyn Storage,
        router: &Router<C>,
        block: &BlockInfo,
        sender: T,
        recipient: String,
        amount: &[Coin],
    ) -> Result<AppResponse, String> {
        if !amount.is_empty() {
            let msg = BankMsg::Send {
                to_address: recipient,
                amount: amount.to_vec(),
            };
            let res = router.execute(storage, block, sender.into(), msg.into())?;
            Ok(res)
        } else {
            Ok(AppResponse::default())
        }
    }

    // this returns the contract address as well, so we can properly resend the data
    fn execute_wasm(
        &self,
        storage: &mut dyn Storage,
        router: &Router<C>,
        block: &BlockInfo,
        sender: Addr,
        msg: WasmMsg,
    ) -> Result<(Addr, Response<C>), String> {
        match msg {
            WasmMsg::Execute {
                contract_addr,
                msg,
                funds,
            } => {
                let contract_addr = Addr::unchecked(contract_addr);
                // first move the cash
                self.send(
                    storage,
                    router,
                    block,
                    sender.clone(),
                    contract_addr.clone().into(),
                    &funds,
                )?;

                // then call the contract
                let info = MessageInfo { sender, funds };
                let res = self.call_execute(
                    storage,
                    contract_addr.clone(),
                    router,
                    block,
                    info,
                    msg.to_vec(),
                )?;
                Ok((contract_addr, res))
            }
            WasmMsg::Instantiate {
                admin: _,
                code_id,
                msg,
                funds,
                label: _,
            } => {
                let contract_addr =
                    Addr::unchecked(self.register_contract(storage, code_id as usize)?);
                // move the cash
                self.send(
                    storage,
                    router,
                    block,
                    sender.clone(),
                    contract_addr.clone().into(),
                    &funds,
                )?;

                // then call the contract
                let info = MessageInfo { sender, funds };
                let mut res = self.call_instantiate(
                    contract_addr.clone(),
                    storage,
                    router,
                    block,
                    info,
                    msg.to_vec(),
                )?;
                init_response(&mut res, &contract_addr);
                Ok((contract_addr, res))
            }
            WasmMsg::Migrate { .. } => unimplemented!(),
            m => panic!("Unsupported wasm message: {:?}", m),
        }
    }

    /// This will execute the given messages, making all changes to the local cache.
    /// This *will* write some data to the cache if the message fails half-way through.
    /// All sequential calls to RouterCache will be one atomic unit (all commit or all fail).
    ///
    /// For normal use cases, you can use Router::execute() or Router::execute_multi().
    /// This is designed to be handled internally as part of larger process flows.
    fn execute_submsg(
        &self,
        router: &Router<C>,
        storage: &mut dyn Storage,
        block: &BlockInfo,
        contract: Addr,
        msg: SubMsg<C>,
    ) -> Result<AppResponse, String> {
        let SubMsg {
            msg, id, reply_on, ..
        } = msg;

        // execute in cache
        let res = transactional(storage, |write_cache, _| {
            router.execute(write_cache, block, contract.clone(), msg)
        });

        // call reply if meaningful
        if let Ok(r) = res {
            if matches!(reply_on, ReplyOn::Always | ReplyOn::Success) {
                let mut orig = r.clone();
                let reply = Reply {
                    id,
                    result: ContractResult::Ok(SubMsgExecutionResponse {
                        events: r.events,
                        data: r.data,
                    }),
                };
                // do reply and combine it with the original response
                let res2 = self._reply(router, storage, block, contract, reply)?;
                // override data if set
                if let Some(data) = res2.data {
                    orig.data = Some(data);
                }
                // append the events
                orig.events.extend_from_slice(&res2.events);
                Ok(orig)
            } else {
                Ok(r)
            }
        } else if let Err(e) = res {
            if matches!(reply_on, ReplyOn::Always | ReplyOn::Error) {
                let reply = Reply {
                    id,
                    result: ContractResult::Err(e),
                };
                self._reply(router, storage, block, contract, reply)
            } else {
                Err(e)
            }
        } else {
            res
        }
    }

    fn _reply(
        &self,
        router: &Router<C>,
        storage: &mut dyn Storage,
        block: &BlockInfo,
        contract: Addr,
        reply: Reply,
    ) -> Result<AppResponse, String> {
        let res = self.call_reply(contract.clone(), storage, router, block, reply)?;
        // TODO: process result better, combine events / data from parent
        self.process_response(router, storage, block, contract, res, true)
    }

    fn process_response(
        &self,
        router: &Router<C>,
        storage: &mut dyn Storage,
        block: &BlockInfo,
        contract: Addr,
        response: Response<C>,
        ignore_attributes: bool,
    ) -> Result<AppResponse, String> {
        // These need to get `wasm-` prefix to match the wasmd semantics (custom wasm messages cannot
        // fake system level event types, like transfer from the bank module)
        let mut events: Vec<_> = response
            .events
            .into_iter()
            .map(|mut ev| {
                ev.ty = format!("wasm-{}", ev.ty);
                ev
            })
            .collect();
        // hmmm... we don't need this for reply, right?
        if !ignore_attributes {
            // turn attributes into event and place it first
            let mut wasm_event = Event::new("wasm").add_attribute("contract_address", &contract);
            wasm_event
                .attributes
                .extend_from_slice(&response.attributes);
            events.insert(0, wasm_event);
        }

        // recurse in all messages
        for resend in response.messages {
            let subres = self.execute_submsg(router, storage, block, contract.clone(), resend)?;
            events.extend_from_slice(&subres.events);
        }
        Ok(AppResponse {
            events,
            data: response.data,
        })
    }

    /// This just creates an address and empty storage instance, returning the new address
    /// You must call init after this to set up the contract properly.
    /// These are separated into two steps to have cleaner return values.
    pub fn register_contract(
        &self,
        storage: &mut dyn Storage,
        code_id: usize,
    ) -> Result<Addr, String> {
        let mut wasm_storage = prefixed(storage, NAMESPACE_WASM);

        if !self.codes.contains_key(&code_id) {
            return Err("Cannot init contract with unregistered code id".to_string());
        }
        let addr = self.next_address(&wasm_storage);
        let info = ContractData::new(code_id);
        CONTRACTS
            .save(&mut wasm_storage, &addr, &info)
            .map_err(|e| e.to_string())?;
        Ok(addr)
    }

    pub fn call_execute(
        &self,
        storage: &mut dyn Storage,
        address: Addr,
        router: &Router<C>,
        block: &BlockInfo,
        info: MessageInfo,
        msg: Vec<u8>,
    ) -> Result<Response<C>, String> {
        self.with_storage(storage, router, block, address, |contract, deps, env| {
            contract.execute(deps, env, info, msg)
        })
    }

    pub fn call_instantiate(
        &self,
        address: Addr,
        storage: &mut dyn Storage,
        router: &Router<C>,
        block: &BlockInfo,
        info: MessageInfo,
        msg: Vec<u8>,
    ) -> Result<Response<C>, String> {
        self.with_storage(storage, router, block, address, |contract, deps, env| {
            contract.instantiate(deps, env, info, msg)
        })
    }

    pub fn call_reply(
        &self,
        address: Addr,
        storage: &mut dyn Storage,
        router: &Router<C>,
        block: &BlockInfo,
        reply: Reply,
    ) -> Result<Response<C>, String> {
        self.with_storage(storage, router, block, address, |contract, deps, env| {
            contract.reply(deps, env, reply)
        })
    }

    pub fn call_sudo(
        &self,
        address: Addr,
        storage: &mut dyn Storage,
        router: &Router<C>,
        block: &BlockInfo,
        msg: Vec<u8>,
    ) -> Result<Response<C>, String> {
        self.with_storage(storage, router, block, address, |contract, deps, env| {
            contract.sudo(deps, env, msg)
        })
    }

    fn get_env<T: Into<Addr>>(&self, address: T, block: &BlockInfo) -> Env {
        Env {
            block: block.clone(),
            contract: ContractInfo {
                address: address.into(),
            },
        }
    }

    fn with_storage_readonly<F, T>(
        &self,
        storage: &dyn Storage,
        querier: &dyn Querier,
        block: &BlockInfo,
        address: Addr,
        action: F,
    ) -> Result<T, String>
    where
        F: FnOnce(&Box<dyn Contract<C>>, Deps, Env) -> Result<T, String>,
    {
        let contract = self.load_contract(storage, &address)?;
        let handler = self
            .codes
            .get(&contract.code_id)
            .ok_or_else(|| "Unregistered code id".to_string())?;
        let storage = self.contract_storage_readonly(storage, &address);
        let env = self.get_env(address, block);

        let deps = Deps {
            storage: storage.as_ref(),
            api: self.api.deref(),
            querier: QuerierWrapper::new(querier),
        };
        action(handler, deps, env)
    }

    fn with_storage<F, T>(
        &self,
        storage: &mut dyn Storage,
        router: &Router<C>,
        block: &BlockInfo,
        address: Addr,
        action: F,
    ) -> Result<T, String>
    where
        F: FnOnce(&Box<dyn Contract<C>>, DepsMut, Env) -> Result<T, String>,
    {
        let contract = self.load_contract(storage, &address)?;
        let handler = self
            .codes
            .get(&contract.code_id)
            .ok_or_else(|| "Unregistered code id".to_string())?;

        transactional(storage, |write_cache, read_store| {
            let mut contract_storage = self.contract_storage(write_cache, &address);
            let querier = RouterQuerier::new(router, read_store, block);
            let env = self.get_env(address, block);

            let deps = DepsMut {
                storage: contract_storage.as_mut(),
                api: self.api.deref(),
                querier: QuerierWrapper::new(&querier),
            };
            action(handler, deps, env)
        })
    }

    fn load_contract(&self, storage: &dyn Storage, address: &Addr) -> Result<ContractData, String> {
        CONTRACTS
            .load(&prefixed_read(storage, NAMESPACE_WASM), address)
            .map_err(|e| e.to_string())
    }

    // FIXME: better addr generation
    fn next_address(&self, storage: &dyn Storage) -> Addr {
        // FIXME: quite inefficient if we actually had 100s of contracts
        let count = CONTRACTS
            .range(storage, None, None, Order::Ascending)
            .count();
        // we make this longer so it is not rejected by tests
        Addr::unchecked(format!("Contract #{}", count.to_string()))
    }

    fn contract_namespace(&self, contract: &Addr) -> Vec<u8> {
        let mut name = b"contract_data/".to_vec();
        name.extend_from_slice(contract.as_bytes());
        name
    }

    fn contract_storage<'a>(
        &self,
        storage: &'a mut dyn Storage,
        address: &Addr,
    ) -> Box<dyn Storage + 'a> {
        // We double-namespace this, once from global storage -> wasm_storage
        // then from wasm_storage -> the contracts subspace
        let namespace = self.contract_namespace(address);
        let storage = PrefixedStorage::multilevel(storage, &[NAMESPACE_WASM, &namespace]);
        Box::new(storage)
    }

    // fails RUNTIME if you try to write. please don't
    fn contract_storage_readonly<'a>(
        &self,
        storage: &'a dyn Storage,
        address: &Addr,
    ) -> Box<dyn Storage + 'a> {
        // We double-namespace this, once from global storage -> wasm_storage
        // then from wasm_storage -> the contracts subspace
        let namespace = self.contract_namespace(address);
        let storage = ReadonlyPrefixedStorage::multilevel(storage, &[NAMESPACE_WASM, &namespace]);
        Box::new(storage)
    }
}

#[derive(Clone, PartialEq, Message)]
pub struct InstantiateData {
    #[prost(string, tag = "1")]
    pub address: ::prost::alloc::string::String,
    /// Unique ID number for this person.
    #[prost(bytes, tag = "2")]
    pub data: ::prost::alloc::vec::Vec<u8>,
}

fn init_response<C>(res: &mut Response<C>, contact_address: &Addr)
where
    C: Clone + fmt::Debug + PartialEq + JsonSchema,
{
    let data = res.data.clone().unwrap_or_default().to_vec();
    let init_data = InstantiateData {
        address: contact_address.into(),
        data,
    };
    let mut new_data = Vec::<u8>::with_capacity(init_data.encoded_len());
    // the data must encode successfully
    init_data.encode(&mut new_data).unwrap();
    res.data = Some(new_data.into());
}

// this parses the result from a wasm contract init
pub fn parse_contract_addr(data: &Option<Binary>) -> Result<Addr, String> {
    let bin = data
        .as_ref()
        .ok_or_else(|| "No data response".to_string())?
        .to_vec();
    // parse the protobuf struct
    let init_data = InstantiateData::decode(bin.as_slice()).map_err(|e| e.to_string())?;
    if init_data.address.is_empty() {
        return Err("no contract address provided".into());
    }
    Ok(Addr::unchecked(init_data.address))
}

#[cfg(test)]
mod test {
    use cosmwasm_std::testing::{mock_env, mock_info, MockApi, MockQuerier, MockStorage};
    use cosmwasm_std::{coin, from_slice, to_vec, BankMsg, Coin, CosmosMsg, Empty};

    use crate::test_helpers::{contract_error, contract_payout, PayoutInitMessage, PayoutQueryMsg};
    use crate::transactions::StorageTransaction;
    use crate::BankKeeper;

    use super::*;

    fn mock_keeper() -> WasmKeeper<Empty> {
        let api = Box::new(MockApi::default());
        WasmKeeper::new(api)
    }

    fn mock_router() -> Router<Empty> {
        let api = Box::new(MockApi::default());
        Router::new(api, BankKeeper {})
    }

    #[test]
    fn register_contract() {
        let mut wasm_storage = MockStorage::new();
        let mut keeper = mock_keeper();
        let block = mock_env().block;
        let code_id = keeper.store_code(contract_error());

        let mut cache = StorageTransaction::new(&wasm_storage);

        // cannot register contract with unregistered codeId
        keeper
            .register_contract(&mut cache, code_id + 1)
            .unwrap_err();

        // we can register a new instance of this code
        let contract_addr = keeper.register_contract(&mut cache, code_id).unwrap();

        // now, we call this contract and see the error message from the contract
        let info = mock_info("foobar", &[]);
        let err = keeper
            .call_instantiate(
                contract_addr,
                &mut cache,
                &mock_router(),
                &block,
                info,
                b"{}".to_vec(),
            )
            .unwrap_err();
        // StdError from contract_error auto-converted to string
        assert_eq!(err, "Generic error: Init failed");

        // and the error for calling an unregistered contract
        let info = mock_info("foobar", &[]);
        let err = keeper
            .call_instantiate(
                Addr::unchecked("unregistered"),
                &mut cache,
                &mock_router(),
                &block,
                info,
                b"{}".to_vec(),
            )
            .unwrap_err();
        // Default error message from router when not found
        assert_eq!(err, "cw_multi_test::wasm::ContractData not found");

        // and flush
        cache.prepare().commit(&mut wasm_storage);
    }

    #[test]
    fn contract_send_coins() {
        let mut keeper = mock_keeper();
        let block = mock_env().block;
        let code_id = keeper.store_code(contract_payout());

        let mut wasm_storage = MockStorage::new();
        let mut cache = StorageTransaction::new(&wasm_storage);

        let contract_addr = keeper.register_contract(&mut cache, code_id).unwrap();

        let payout = coin(100, "TGD");

        // init the contract
        let info = mock_info("foobar", &[]);
        let init_msg = to_vec(&PayoutInitMessage {
            payout: payout.clone(),
        })
        .unwrap();
        let res = keeper
            .call_instantiate(
                contract_addr.clone(),
                &mut cache,
                &mock_router(),
                &block,
                info,
                init_msg,
            )
            .unwrap();
        assert_eq!(0, res.messages.len());

        // execute the contract
        let info = mock_info("foobar", &[]);
        let res = keeper
            .call_execute(
                &mut cache,
                contract_addr.clone(),
                &mock_router(),
                &block,
                info,
                b"{}".to_vec(),
            )
            .unwrap();
        assert_eq!(1, res.messages.len());
        match &res.messages[0].msg {
            CosmosMsg::Bank(BankMsg::Send { to_address, amount }) => {
                assert_eq!(to_address.as_str(), "foobar");
                assert_eq!(amount.as_slice(), &[payout.clone()]);
            }
            m => panic!("Unexpected message {:?}", m),
        }

        // and flush before query
        cache.prepare().commit(&mut wasm_storage);

        // query the contract
        let query = to_vec(&PayoutQueryMsg::Payout {}).unwrap();
        let querier: MockQuerier<Empty> = MockQuerier::new(&[]);
        let data = keeper
            .query_smart(contract_addr, &wasm_storage, &querier, &block, query)
            .unwrap();
        let res: PayoutInitMessage = from_slice(&data).unwrap();
        assert_eq!(res.payout, payout);
    }

    fn assert_payout(
        router: &WasmKeeper<Empty>,
        storage: &mut dyn Storage,
        contract_addr: &Addr,
        payout: &Coin,
    ) {
        let info = mock_info("silly", &[]);
        let res = router
            .call_execute(
                storage,
                contract_addr.clone(),
                &mock_router(),
                &mock_env().block,
                info,
                b"{}".to_vec(),
            )
            .unwrap();
        assert_eq!(1, res.messages.len());
        match &res.messages[0].msg {
            CosmosMsg::Bank(BankMsg::Send { to_address, amount }) => {
                assert_eq!(to_address.as_str(), "silly");
                assert_eq!(amount.as_slice(), &[payout.clone()]);
            }
            m => panic!("Unexpected message {:?}", m),
        }
    }

    fn assert_no_contract(storage: &dyn Storage, contract_addr: &Addr) {
        let contract = CONTRACTS.may_load(storage, contract_addr).unwrap();
        assert!(contract.is_none(), "{:?}", contract_addr);
    }

    #[test]
    fn multi_level_wasm_cache() {
        let mut keeper = mock_keeper();
        let block = mock_env().block;
        let code_id = keeper.store_code(contract_payout());

        let mut wasm_storage = MockStorage::new();
        let mut cache = StorageTransaction::new(&wasm_storage);

        // set contract 1 and commit (on router)
        let contract1 = keeper.register_contract(&mut cache, code_id).unwrap();
        let payout1 = coin(100, "TGD");
        let info = mock_info("foobar", &[]);
        let init_msg = to_vec(&PayoutInitMessage {
            payout: payout1.clone(),
        })
        .unwrap();
        let _res = keeper
            .call_instantiate(
                contract1.clone(),
                &mut cache,
                &mock_router(),
                &block,
                info,
                init_msg,
            )
            .unwrap();
        cache.prepare().commit(&mut wasm_storage);

        // create a new cache and check we can use contract 1
        let mut cache = StorageTransaction::new(&wasm_storage);
        assert_payout(&keeper, &mut cache, &contract1, &payout1);

        // create contract 2 and use it
        let contract2 = keeper.register_contract(&mut cache, code_id).unwrap();
        let payout2 = coin(50, "BTC");
        let info = mock_info("foobar", &[]);
        let init_msg = to_vec(&PayoutInitMessage {
            payout: payout2.clone(),
        })
        .unwrap();
        let _res = keeper
            .call_instantiate(
                contract2.clone(),
                &mut cache,
                &mock_router(),
                &block,
                info,
                init_msg,
            )
            .unwrap();
        assert_payout(&keeper, &mut cache, &contract2, &payout2);

        // create a level2 cache and check we can use contract 1 and contract 2
        let mut cache2 = cache.cache();
        assert_payout(&keeper, &mut cache2, &contract1, &payout1);
        assert_payout(&keeper, &mut cache2, &contract2, &payout2);

        // create a contract on level 2
        let contract3 = keeper.register_contract(&mut cache2, code_id).unwrap();
        let payout3 = coin(1234, "ATOM");
        let info = mock_info("johnny", &[]);
        let init_msg = to_vec(&PayoutInitMessage {
            payout: payout3.clone(),
        })
        .unwrap();
        let _res = keeper
            .call_instantiate(
                contract3.clone(),
                &mut cache2,
                &mock_router(),
                &block,
                info,
                init_msg,
            )
            .unwrap();
        assert_payout(&keeper, &mut cache2, &contract3, &payout3);

        // ensure first cache still doesn't see this contract
        assert_no_contract(&cache, &contract3);

        // apply second to first, all contracts present
        cache2.prepare().commit(&mut cache);
        assert_payout(&keeper, &mut cache, &contract1, &payout1);
        assert_payout(&keeper, &mut cache, &contract2, &payout2);
        assert_payout(&keeper, &mut cache, &contract3, &payout3);

        // apply to router
        cache.prepare().commit(&mut wasm_storage);

        // make new cache and see all contracts there
        assert_payout(&keeper, &mut wasm_storage, &contract1, &payout1);
        assert_payout(&keeper, &mut wasm_storage, &contract2, &payout2);
        assert_payout(&keeper, &mut wasm_storage, &contract3, &payout3);
    }
}
