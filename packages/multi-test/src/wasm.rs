use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::ops::Deref;

use cosmwasm_std::{
    Addr, Api, Binary, BlockInfo, ContractInfo, Deps, DepsMut, Env, MessageInfo, Order, Querier,
    QuerierWrapper, Reply, Response, Storage, WasmQuery,
};
use cosmwasm_storage::{prefixed, prefixed_read};
use cw_storage_plus::Map;

use crate::contracts::Contract;

// Contracts is in storage (from Router, or from Cache)
const CONTRACTS: Map<&Addr, ContractData> = Map::new("contracts");

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

pub fn next_block(block: &mut BlockInfo) {
    block.time = block.time.plus_seconds(5);
    block.height += 1;
}

pub struct WasmKeeper<C>
where
    C: Clone + fmt::Debug + PartialEq + JsonSchema,
{
    /// code is in-memory lookup that stands in for wasm code
    /// this can only be editted on the WasmRouter, and just read in caches
    codes: HashMap<usize, Box<dyn Contract<C>>>,

    // WasmConst
    block: BlockInfo,
    api: Box<dyn Api>,
}

impl<C> WasmKeeper<C>
where
    C: Clone + fmt::Debug + PartialEq + JsonSchema,
{
    pub fn new(api: Box<dyn Api>, block: BlockInfo) -> Self {
        WasmKeeper {
            codes: HashMap::new(),
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

    /// Returns a copy of the current block_info
    pub fn block_info(&self) -> BlockInfo {
        self.block.clone()
    }

    // Add a new contract. Must be done on the base object, when no contracts running
    pub fn store_code(&mut self, code: Box<dyn Contract<C>>) -> usize {
        let idx = self.codes.len() + 1;
        self.codes.insert(idx, code);
        idx
    }

    pub fn query(
        &self,
        storage: &dyn Storage,
        querier: &dyn Querier,
        request: WasmQuery,
    ) -> Result<Binary, String> {
        match request {
            WasmQuery::Smart { contract_addr, msg } => {
                self.query_smart(storage, Addr::unchecked(contract_addr), querier, msg.into())
            }
            WasmQuery::Raw { contract_addr, key } => {
                Ok(self.query_raw(storage, Addr::unchecked(contract_addr), &key))
            }
            q => panic!("Unsupported wasm query: {:?}", q),
        }
    }

    pub fn query_smart(
        &self,
        storage: &dyn Storage,
        address: Addr,
        querier: &dyn Querier,
        msg: Vec<u8>,
    ) -> Result<Binary, String> {
        self.with_storage_readonly(storage, querier, address, |handler, deps, env| {
            handler.query(deps, env, msg)
        })
    }

    pub fn query_raw(&self, storage: &dyn Storage, address: Addr, key: &[u8]) -> Binary {
        let storage = self.contract_storage_readonly(storage, &address);
        let data = storage.get(&key).unwrap_or_default();
        data.into()
    }

    /// This just creates an address and empty storage instance, returning the new address
    /// You must call init after this to set up the contract properly.
    /// These are separated into two steps to have cleaner return values.
    pub fn register_contract(
        &self,
        storage: &mut dyn Storage,
        code_id: usize,
    ) -> Result<Addr, String> {
        if !self.codes.contains_key(&code_id) {
            return Err("Cannot init contract with unregistered code id".to_string());
        }
        let addr = self.next_address(storage);
        let info = ContractData::new(code_id);
        CONTRACTS
            .save(storage, &addr, &info)
            .map_err(|e| e.to_string())?;
        Ok(addr)
    }

    pub fn execute(
        &self,
        storage: &mut dyn Storage,
        address: Addr,
        querier: &dyn Querier,
        info: MessageInfo,
        msg: Vec<u8>,
    ) -> Result<Response<C>, String> {
        self.with_storage(storage, querier, address, |contract, deps, env| {
            contract.execute(deps, env, info, msg)
        })
    }

    pub fn instantiate(
        &self,
        storage: &mut dyn Storage,
        address: Addr,
        querier: &dyn Querier,
        info: MessageInfo,
        msg: Vec<u8>,
    ) -> Result<Response<C>, String> {
        self.with_storage(storage, querier, address, |contract, deps, env| {
            contract.instantiate(deps, env, info, msg)
        })
    }

    pub fn reply(
        &self,
        storage: &mut dyn Storage,
        address: Addr,
        querier: &dyn Querier,
        reply: Reply,
    ) -> Result<Response<C>, String> {
        self.with_storage(storage, querier, address, |contract, deps, env| {
            contract.reply(deps, env, reply)
        })
    }

    pub fn sudo(
        &self,
        storage: &mut dyn Storage,
        address: Addr,
        querier: &dyn Querier,
        msg: Vec<u8>,
    ) -> Result<Response<C>, String> {
        self.with_storage(storage, querier, address, |contract, deps, env| {
            contract.sudo(deps, env, msg)
        })
    }

    fn get_env<T: Into<Addr>>(&self, address: T) -> Env {
        Env {
            block: self.block.clone(),
            contract: ContractInfo {
                address: address.into(),
            },
        }
    }

    fn with_storage_readonly<F, T>(
        &self,
        storage: &dyn Storage,
        querier: &dyn Querier,
        address: Addr,
        action: F,
    ) -> Result<T, String>
    where
        F: FnOnce(&Box<dyn Contract<C>>, Deps, Env) -> Result<T, String>,
    {
        let contract = CONTRACTS
            .load(storage, &address)
            .map_err(|e| e.to_string())?;
        let handler = self
            .codes
            .get(&contract.code_id)
            .ok_or_else(|| "Unregistered code id".to_string())?;
        let storage = self.contract_storage_readonly(storage, &address);
        let env = self.get_env(address);

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
        querier: &dyn Querier,
        address: Addr,
        action: F,
    ) -> Result<T, String>
    where
        F: FnOnce(&Box<dyn Contract<C>>, DepsMut, Env) -> Result<T, String>,
    {
        let contract = CONTRACTS
            .load(storage, &address)
            .map_err(|e| e.to_string())?;
        let handler = self
            .codes
            .get(&contract.code_id)
            .ok_or_else(|| "Unregistered code id".to_string())?;
        let mut storage = self.contract_storage(storage, &address);
        let env = self.get_env(address);

        let deps = DepsMut {
            storage: storage.as_mut(),
            api: self.api.deref(),
            querier: QuerierWrapper::new(querier),
        };
        action(handler, deps, env)
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
        // FIXME: revisit how we namespace? (no conflicts with CONTRACTS, other?)
        let mut name = b"contract_data".to_vec();
        name.extend_from_slice(contract.as_bytes());
        name
    }

    fn contract_storage<'a>(
        &self,
        storage: &'a mut dyn Storage,
        address: &Addr,
    ) -> Box<dyn Storage + 'a> {
        let namespace = self.contract_namespace(address);
        let storage = prefixed(storage, &namespace);
        Box::new(storage)
    }

    // fails RUNTIME if you try to write. please don't
    fn contract_storage_readonly<'a>(
        &self,
        storage: &'a dyn Storage,
        address: &Addr,
    ) -> Box<dyn Storage + 'a> {
        let namespace = self.contract_namespace(address);
        let storage = prefixed_read(storage, &namespace);
        Box::new(storage)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use crate::test_helpers::{contract_error, contract_payout, PayoutInitMessage, PayoutQueryMsg};
    use crate::transactions::StorageTransaction;
    use cosmwasm_std::testing::{mock_env, mock_info, MockApi, MockQuerier, MockStorage};
    use cosmwasm_std::{coin, from_slice, to_vec, BankMsg, BlockInfo, Coin, CosmosMsg, Empty};

    fn mock_router() -> WasmKeeper<Empty> {
        let env = mock_env();
        let api = Box::new(MockApi::default());
        WasmKeeper::new(api, env.block)
    }

    #[test]
    fn register_contract() {
        let mut wasm_storage = MockStorage::new();
        let mut router = mock_router();
        let code_id = router.store_code(contract_error());

        let mut cache = StorageTransaction::new(&wasm_storage);

        // cannot register contract with unregistered codeId
        router
            .register_contract(&mut cache, code_id + 1)
            .unwrap_err();

        // we can register a new instance of this code
        let contract_addr = router.register_contract(&mut cache, code_id).unwrap();

        // now, we call this contract and see the error message from the contract
        let querier: MockQuerier<Empty> = MockQuerier::new(&[]);
        let info = mock_info("foobar", &[]);
        let err = router
            .instantiate(&mut cache, contract_addr, &querier, info, b"{}".to_vec())
            .unwrap_err();
        // StdError from contract_error auto-converted to string
        assert_eq!(err, "Generic error: Init failed");

        // and the error for calling an unregistered contract
        let info = mock_info("foobar", &[]);
        let err = router
            .instantiate(
                &mut cache,
                Addr::unchecked("unregistered"),
                &querier,
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
    fn update_block() {
        let mut router = mock_router();

        let BlockInfo { time, height, .. } = router.get_env(Addr::unchecked("foo")).block;
        router.update_block(next_block);
        let next = router.get_env(Addr::unchecked("foo")).block;

        assert_eq!(time.plus_seconds(5), next.time);
        assert_eq!(height + 1, next.height);
    }

    #[test]
    fn contract_send_coins() {
        let mut router = mock_router();
        let code_id = router.store_code(contract_payout());

        let mut wasm_storage = MockStorage::new();
        let mut cache = StorageTransaction::new(&wasm_storage);

        let contract_addr = router.register_contract(&mut cache, code_id).unwrap();

        let querier: MockQuerier<Empty> = MockQuerier::new(&[]);
        let payout = coin(100, "TGD");

        // init the contract
        let info = mock_info("foobar", &[]);
        let init_msg = to_vec(&PayoutInitMessage {
            payout: payout.clone(),
        })
        .unwrap();
        let res = router
            .instantiate(&mut cache, contract_addr.clone(), &querier, info, init_msg)
            .unwrap();
        assert_eq!(0, res.messages.len());

        // execute the contract
        let info = mock_info("foobar", &[]);
        let res = router
            .execute(
                &mut cache,
                contract_addr.clone(),
                &querier,
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
        let data = router
            .query_smart(&wasm_storage, contract_addr, &querier, query)
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
        let querier: MockQuerier<Empty> = MockQuerier::new(&[]);
        let info = mock_info("silly", &[]);
        let res = router
            .execute(
                storage,
                contract_addr.clone(),
                &querier,
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
        let mut router = mock_router();
        let code_id = router.store_code(contract_payout());
        let querier: MockQuerier<Empty> = MockQuerier::new(&[]);

        let mut wasm_storage = MockStorage::new();
        let mut cache = StorageTransaction::new(&wasm_storage);

        // set contract 1 and commit (on router)
        let contract1 = router.register_contract(&mut cache, code_id).unwrap();
        let payout1 = coin(100, "TGD");
        let info = mock_info("foobar", &[]);
        let init_msg = to_vec(&PayoutInitMessage {
            payout: payout1.clone(),
        })
        .unwrap();
        let _res = router
            .instantiate(&mut cache, contract1.clone(), &querier, info, init_msg)
            .unwrap();
        cache.prepare().commit(&mut wasm_storage);

        // create a new cache and check we can use contract 1
        let mut cache = StorageTransaction::new(&wasm_storage);
        assert_payout(&router, &mut cache, &contract1, &payout1);

        // create contract 2 and use it
        let contract2 = router.register_contract(&mut cache, code_id).unwrap();
        let payout2 = coin(50, "BTC");
        let info = mock_info("foobar", &[]);
        let init_msg = to_vec(&PayoutInitMessage {
            payout: payout2.clone(),
        })
        .unwrap();
        let _res = router
            .instantiate(&mut cache, contract2.clone(), &querier, info, init_msg)
            .unwrap();
        assert_payout(&router, &mut cache, &contract2, &payout2);

        // create a level2 cache and check we can use contract 1 and contract 2
        let mut cache2 = cache.cache();
        assert_payout(&router, &mut cache2, &contract1, &payout1);
        assert_payout(&router, &mut cache2, &contract2, &payout2);

        // create a contract on level 2
        let contract3 = router.register_contract(&mut cache2, code_id).unwrap();
        let payout3 = coin(1234, "ATOM");
        let info = mock_info("johnny", &[]);
        let init_msg = to_vec(&PayoutInitMessage {
            payout: payout3.clone(),
        })
        .unwrap();
        let _res = router
            .instantiate(&mut cache2, contract3.clone(), &querier, info, init_msg)
            .unwrap();
        assert_payout(&router, &mut cache2, &contract3, &payout3);

        // ensure first cache still doesn't see this contract
        assert_no_contract(&cache, &contract3);

        // apply second to first, all contracts present
        cache2.prepare().commit(&mut cache);
        assert_payout(&router, &mut cache, &contract1, &payout1);
        assert_payout(&router, &mut cache, &contract2, &payout2);
        assert_payout(&router, &mut cache, &contract3, &payout3);

        // apply to router
        cache.prepare().commit(&mut wasm_storage);

        // make new cache and see all contracts there
        assert_payout(&router, &mut wasm_storage, &contract1, &payout1);
        assert_payout(&router, &mut wasm_storage, &contract2, &payout2);
        assert_payout(&router, &mut wasm_storage, &contract3, &payout3);
    }
}
