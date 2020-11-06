use serde::Serialize;

#[cfg(test)]
use cosmwasm_std::testing::{mock_env, MockApi};
use cosmwasm_std::{
    from_slice, to_binary, Api, Attribute, BankMsg, Binary, BlockInfo, Coin, ContractResult,
    CosmosMsg, Empty, HandleResponse, HumanAddr, InitResponse, MessageInfo, Querier, QuerierResult,
    QuerierWrapper, QueryRequest, SystemError, SystemResult, WasmMsg,
};

use crate::bank::{Bank, BankCache, BankOps, BankRouter};
use crate::wasm::{Contract, StorageFactory, WasmCache, WasmOps, WasmRouter};

#[derive(Default, Clone, Debug)]
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

/// Router is a persisted state. You can query this.
/// Execution generally happens on the RouterCache, which then can be atomically committed or rolled back.
/// We offer .execute() as a wrapper around cache, execute, commit/rollback process
pub struct Router {
    wasm: WasmRouter,
    bank: BankRouter,
}

impl Querier for Router {
    fn raw_query(&self, bin_request: &[u8]) -> QuerierResult {
        let request: QueryRequest<Empty> = match from_slice(bin_request) {
            Ok(v) => v,
            Err(e) => {
                return SystemResult::Err(SystemError::InvalidRequest {
                    error: format!("Parsing query request: {}", e.to_string()),
                    request: bin_request.into(),
                })
            }
        };
        let contract_result: ContractResult<Binary> = self.query(request).into();
        SystemResult::Ok(contract_result)
    }
}

impl Router {
    pub fn new<B: Bank + 'static>(
        api: Box<dyn Api>,
        block: BlockInfo,
        bank: B,
        storage_factory: StorageFactory,
    ) -> Self {
        Router {
            wasm: WasmRouter::new(api, block, storage_factory),
            bank: BankRouter::new(bank, storage_factory()),
        }
    }

    pub fn cache(&'_ self) -> RouterCache<'_> {
        RouterCache::new(self)
    }

    /// This can set the block info to any value. Must be done before taking a cache
    pub fn set_block(&mut self, block: BlockInfo) {
        self.wasm.set_block(block);
    }

    /// This let's use use "next block" steps that add eg. one height and 5 seconds
    pub fn update_block<F: Fn(&mut BlockInfo)>(&mut self, action: F) {
        self.wasm.update_block(action);
    }

    /// This is an "admin" function to let us adjust bank accounts
    pub fn set_bank_balance(
        &mut self,
        account: HumanAddr,
        amount: Vec<Coin>,
    ) -> Result<(), String> {
        self.bank.set_balance(account, amount)
    }

    /// This registers contract code (like uploading wasm bytecode on a chain),
    /// so it can later be used to instantiate a contract.
    pub fn store_code(&mut self, code: Box<dyn Contract>) -> u64 {
        self.wasm.store_code(code) as u64
    }

    /// Simple helper so we get access to all the QuerierWrapper helpers,
    /// eg. query_wasm_smart, query_all_balances, ...
    pub fn wrap(&self) -> QuerierWrapper {
        QuerierWrapper::new(self)
    }

    /// Handles arbitrary QueryRequest, this is wrapped by the Querier interface, but this
    /// is nicer to use.
    pub fn query(&self, request: QueryRequest<Empty>) -> Result<Binary, String> {
        match request {
            QueryRequest::Wasm(req) => self.wasm.query(self, req),
            QueryRequest::Bank(req) => self.bank.query(req),
            _ => unimplemented!(),
        }
    }

    /// Create a contract and get the new address.
    /// This is just a helper around execute()
    pub fn instantiate_contract<T: Serialize, U: Into<String>, V: Into<HumanAddr>>(
        &mut self,
        code_id: u64,
        sender: V,
        init_msg: &T,
        send_funds: &[Coin],
        label: U,
    ) -> Result<HumanAddr, String> {
        // instantiate contract
        let init_msg = to_binary(init_msg).map_err(|e| e.to_string())?;
        let msg: CosmosMsg = WasmMsg::Instantiate {
            code_id,
            msg: init_msg,
            send: send_funds.to_vec(),
            label: Some(label.into()),
        }
        .into();
        let res = self.execute(sender.into(), msg)?;
        parse_contract_addr(&res.data)
    }

    /// Execute a contract and process all returned messages.
    /// This is just a helper around execute()
    pub fn execute_contract<T: Serialize, U: Into<HumanAddr>>(
        &mut self,
        contract_addr: U,
        sender: U,
        msg: &T,
        send_funds: &[Coin],
    ) -> Result<RouterResponse, String> {
        let msg = to_binary(msg).map_err(|e| e.to_string())?;
        let msg = WasmMsg::Execute {
            contract_addr: contract_addr.into(),
            msg,
            send: send_funds.to_vec(),
        }
        .into();
        self.execute(sender.into(), msg)
    }

    /// Runs arbitrary CosmosMsg.
    /// This will create a cache before the execution, so no state changes are persisted if this
    /// returns an error, but all are persisted on success.
    pub fn execute(
        &mut self,
        sender: HumanAddr,
        msg: CosmosMsg<Empty>,
    ) -> Result<RouterResponse, String> {
        let mut all = self.execute_multi(sender, vec![msg])?;
        let res = all.pop().unwrap();
        Ok(res)
    }

    /// Runs multiple CosmosMsg in one atomic operation.
    /// This will create a cache before the execution, so no state changes are persisted if any of them
    /// return an error. But all writes are persisted on success.
    pub fn execute_multi(
        &mut self,
        sender: HumanAddr,
        msgs: Vec<CosmosMsg<Empty>>,
    ) -> Result<Vec<RouterResponse>, String> {
        // we need to do some caching of storage here, once in the entry point:
        // meaning, wrap current state, all writes go to a cache, only when execute
        // returns a success do we flush it (otherwise drop it)

        let mut cache = self.cache();

        // run all messages, stops at first error
        let res: Result<Vec<RouterResponse>, String> = msgs
            .into_iter()
            .map(|msg| cache.execute(sender.clone(), msg))
            .collect();

        // this only happens if all messages run successfully
        if res.is_ok() {
            let ops = cache.prepare();
            ops.commit(self);
        }
        res
    }
}

pub struct RouterCache<'a> {
    router: &'a Router,
    wasm: WasmCache<'a>,
    bank: BankCache<'a>,
}

pub struct RouterOps {
    wasm: WasmOps,
    bank: BankOps,
}

impl RouterOps {
    pub fn commit(self, router: &mut Router) {
        self.bank.commit(&mut router.bank);
        self.wasm.commit(&mut router.wasm);
    }
}

impl<'a> RouterCache<'a> {
    fn new(router: &'a Router) -> Self {
        RouterCache {
            router,
            wasm: router.wasm.cache(),
            bank: router.bank.cache(),
        }
    }

    /// When we want to commit the RouterCache, we need a 2 step process to satisfy Rust reference counting:
    /// 1. prepare() consumes RouterCache, releasing &Router, and creating a self-owned update info.
    /// 2. RouterOps::commit() can now take &mut Router and updates the underlying state
    pub fn prepare(self) -> RouterOps {
        RouterOps {
            wasm: self.wasm.prepare(),
            bank: self.bank.prepare(),
        }
    }

    /// This will execute the given messages, making all changes to the local cache.
    /// This *will* write some data to the cache if the message fails half-way through.
    /// All sequential calls to RouterCache will be one atomic unit (all commit or all fail).
    ///
    /// For normal use cases, you can use Router::execute() or Router::execute_multi().
    /// This is designed to be handled internally as part of larger process flows.
    fn execute(
        &mut self,
        sender: HumanAddr,
        msg: CosmosMsg<Empty>,
    ) -> Result<RouterResponse, String> {
        match msg {
            CosmosMsg::Wasm(msg) => {
                let (resender, res) = self.handle_wasm(sender, msg)?;
                let mut attributes = res.attributes;
                // recurse in all messages
                for resend in res.messages {
                    let subres = self.execute(resender.clone(), resend)?;
                    // ignore the data now, just like in wasmd
                    // append the events
                    attributes.extend_from_slice(&subres.attributes);
                }
                Ok(RouterResponse {
                    attributes,
                    data: res.data,
                })
            }
            CosmosMsg::Bank(msg) => {
                self.bank.execute(sender, msg)?;
                Ok(RouterResponse::default())
            }
            _ => unimplemented!(),
        }
    }

    // this returns the contract address as well, so we can properly resend the data
    fn handle_wasm(
        &mut self,
        sender: HumanAddr,
        msg: WasmMsg,
    ) -> Result<(HumanAddr, ActionResponse), String> {
        match msg {
            WasmMsg::Execute {
                contract_addr,
                msg,
                send,
            } => {
                // first move the cash
                self.send(&sender, &contract_addr, &send)?;
                // then call the contract
                let info = MessageInfo {
                    sender,
                    sent_funds: send,
                };
                let res =
                    self.wasm
                        .handle(contract_addr.clone(), self.router, info, msg.to_vec())?;
                Ok((contract_addr, res.into()))
            }
            WasmMsg::Instantiate {
                code_id,
                msg,
                send,
                label: _,
            } => {
                let contract_addr = self.wasm.register_contract(code_id as usize)?;
                // move the cash
                self.send(&sender, &contract_addr, &send)?;
                // then call the contract
                let info = MessageInfo {
                    sender,
                    sent_funds: send,
                };
                let res = self
                    .wasm
                    .init(contract_addr.clone(), self.router, info, msg.to_vec())?;
                Ok((
                    contract_addr.clone(),
                    ActionResponse::init(res, contract_addr),
                ))
            }
        }
    }

    fn send<T: Into<HumanAddr>, U: Into<HumanAddr>>(
        &mut self,
        sender: T,
        recipient: U,
        amount: &[Coin],
    ) -> Result<RouterResponse, String> {
        if !amount.is_empty() {
            let sender: HumanAddr = sender.into();
            let msg = BankMsg::Send {
                from_address: sender.clone(),
                to_address: recipient.into(),
                amount: amount.to_vec(),
            };
            self.bank.execute(sender, msg)?;
        }
        Ok(RouterResponse::default())
    }
}

// this parses the result from a wasm contract init
pub fn parse_contract_addr(data: &Option<Binary>) -> Result<HumanAddr, String> {
    let bin = data
        .as_ref()
        .ok_or_else(|| "No data response".to_string())?
        .to_vec();
    let str = String::from_utf8(bin).map_err(|e| e.to_string())?;
    Ok(HumanAddr::from(str))
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test_helpers::{
        contract_payout, contract_reflect, EmptyMsg, PayoutMessage, ReflectMessage, ReflectResponse,
    };
    use crate::SimpleBank;
    use cosmwasm_std::testing::MockStorage;
    use cosmwasm_std::{attr, coin, coins};

    fn mock_router() -> Router {
        let env = mock_env();
        let api = Box::new(MockApi::default());
        let bank = SimpleBank {};

        Router::new(api, env.block, bank, || Box::new(MockStorage::new()))
    }

    fn get_balance(router: &Router, addr: &HumanAddr) -> Vec<Coin> {
        router.wrap().query_all_balances(addr).unwrap()
    }

    #[test]
    fn send_tokens() {
        let mut router = mock_router();

        let owner = HumanAddr::from("owner");
        let rcpt = HumanAddr::from("receiver");
        let init_funds = vec![coin(20, "btc"), coin(100, "eth")];
        let rcpt_funds = vec![coin(5, "btc")];

        // set money
        router
            .set_bank_balance(owner.clone(), init_funds.clone())
            .unwrap();
        router
            .set_bank_balance(rcpt.clone(), rcpt_funds.clone())
            .unwrap();

        // send both tokens
        let to_send = vec![coin(30, "eth"), coin(5, "btc")];
        let msg: CosmosMsg = BankMsg::Send {
            from_address: owner.clone(),
            to_address: rcpt.clone(),
            amount: to_send.clone(),
        }
        .into();
        router.execute(owner.clone(), msg.clone()).unwrap();
        let rich = get_balance(&router, &owner);
        assert_eq!(vec![coin(15, "btc"), coin(70, "eth")], rich);
        let poor = get_balance(&router, &rcpt);
        assert_eq!(vec![coin(10, "btc"), coin(30, "eth")], poor);

        // cannot send from other account
        router.execute(rcpt.clone(), msg).unwrap_err();

        // cannot send too much
        let msg = BankMsg::Send {
            from_address: owner.clone(),
            to_address: rcpt.clone(),
            amount: coins(20, "btc"),
        }
        .into();
        router.execute(owner.clone(), msg).unwrap_err();

        let rich = get_balance(&router, &owner);
        assert_eq!(vec![coin(15, "btc"), coin(70, "eth")], rich);
    }

    #[test]
    fn simple_contract() {
        let mut router = mock_router();

        // set personal balance
        let owner = HumanAddr::from("owner");
        let init_funds = vec![coin(20, "btc"), coin(100, "eth")];
        router
            .set_bank_balance(owner.clone(), init_funds.clone())
            .unwrap();

        // set up contract
        let code_id = router.store_code(contract_payout());
        let msg = PayoutMessage {
            payout: coin(5, "eth"),
        };
        let contract_addr = router
            .instantiate_contract(code_id, &owner, &msg, &coins(23, "eth"), "Payout")
            .unwrap();

        // sender funds deducted
        let sender = get_balance(&router, &owner);
        assert_eq!(sender, vec![coin(20, "btc"), coin(77, "eth")]);
        // get contract address, has funds
        let funds = get_balance(&router, &contract_addr);
        assert_eq!(funds, coins(23, "eth"));

        // create empty account
        let random = HumanAddr::from("random");
        let funds = get_balance(&router, &random);
        assert_eq!(funds, vec![]);

        // do one payout and see money coming in
        let res = router
            .execute_contract(&contract_addr, &random, &EmptyMsg {}, &[])
            .unwrap();
        assert_eq!(1, res.attributes.len());
        assert_eq!(&attr("action", "payout"), &res.attributes[0]);

        // random got cash
        let funds = get_balance(&router, &random);
        assert_eq!(funds, coins(5, "eth"));
        // contract lost it
        let funds = get_balance(&router, &contract_addr);
        assert_eq!(funds, coins(18, "eth"));
    }

    #[test]
    fn reflect_success() {
        let mut router = mock_router();

        // set personal balance
        let owner = HumanAddr::from("owner");
        let init_funds = vec![coin(20, "btc"), coin(100, "eth")];
        router
            .set_bank_balance(owner.clone(), init_funds.clone())
            .unwrap();

        // set up payout contract
        let payout_id = router.store_code(contract_payout());
        let msg = PayoutMessage {
            payout: coin(5, "eth"),
        };
        let payout_addr = router
            .instantiate_contract(payout_id, &owner, &msg, &coins(23, "eth"), "Payout")
            .unwrap();

        // set up reflect contract
        let reflect_id = router.store_code(contract_reflect());
        let reflect_addr = router
            .instantiate_contract(reflect_id, &owner, &EmptyMsg {}, &[], "Reflect")
            .unwrap();

        // reflect account is empty
        let funds = get_balance(&router, &reflect_addr);
        assert_eq!(funds, vec![]);
        // reflect count is 1
        let qres: ReflectResponse = router
            .wrap()
            .query_wasm_smart(&reflect_addr, &EmptyMsg {})
            .unwrap();
        assert_eq!(1, qres.count);

        // reflecting payout message pays reflect contract
        let msg = WasmMsg::Execute {
            contract_addr: payout_addr.clone(),
            msg: b"{}".into(),
            send: vec![],
        }
        .into();
        let msgs = ReflectMessage {
            messages: vec![msg],
        };
        let res = router
            .execute_contract(&reflect_addr, &HumanAddr::from("random"), &msgs, &[])
            .unwrap();

        // ensure the attributes were relayed from the sub-message
        assert_eq!(1, res.attributes.len());
        assert_eq!(&attr("action", "payout"), &res.attributes[0]);

        // ensure transfer was executed with reflect as sender
        let funds = get_balance(&router, &reflect_addr);
        assert_eq!(funds, coins(5, "eth"));

        // reflect count updated
        let qres: ReflectResponse = router
            .wrap()
            .query_wasm_smart(&reflect_addr, &EmptyMsg {})
            .unwrap();
        assert_eq!(2, qres.count);
    }

    #[test]
    fn reflect_error() {
        let mut router = mock_router();

        // set personal balance
        let owner = HumanAddr::from("owner");
        let init_funds = vec![coin(20, "btc"), coin(100, "eth")];
        router
            .set_bank_balance(owner.clone(), init_funds.clone())
            .unwrap();

        // set up reflect contract
        let reflect_id = router.store_code(contract_reflect());
        let reflect_addr = router
            .instantiate_contract(
                reflect_id,
                &owner,
                &EmptyMsg {},
                &coins(40, "eth"),
                "Reflect",
            )
            .unwrap();

        // reflect has 40 eth
        let funds = get_balance(&router, &reflect_addr);
        assert_eq!(funds, coins(40, "eth"));
        let random = HumanAddr::from("random");

        // sending 7 eth works
        let msg = BankMsg::Send {
            from_address: reflect_addr.clone(),
            to_address: random.clone(),
            amount: coins(7, "eth"),
        }
        .into();
        let msgs = ReflectMessage {
            messages: vec![msg],
        };
        let res = router
            .execute_contract(&reflect_addr, &random, &msgs, &[])
            .unwrap();
        assert_eq!(0, res.attributes.len());
        // ensure random got paid
        let funds = get_balance(&router, &random);
        assert_eq!(funds, coins(7, "eth"));

        // reflect count should be updated to 2
        let qres: ReflectResponse = router
            .wrap()
            .query_wasm_smart(&reflect_addr, &EmptyMsg {})
            .unwrap();
        assert_eq!(2, qres.count);

        // sending 8 eth, then 3 btc should fail both
        let msg = BankMsg::Send {
            from_address: reflect_addr.clone(),
            to_address: random.clone(),
            amount: coins(8, "eth"),
        }
        .into();
        let msg2 = BankMsg::Send {
            from_address: reflect_addr.clone(),
            to_address: random.clone(),
            amount: coins(3, "btc"),
        }
        .into();
        let msgs = ReflectMessage {
            messages: vec![msg, msg2],
        };
        let err = router
            .execute_contract(&reflect_addr, &random, &msgs, &[])
            .unwrap_err();
        assert_eq!("Cannot subtract 3 from 0", err.as_str());

        // first one should have been rolled-back on error (no second payment)
        let funds = get_balance(&router, &random);
        assert_eq!(funds, coins(7, "eth"));

        // failure should not update reflect count
        let qres: ReflectResponse = router
            .wrap()
            .query_wasm_smart(&reflect_addr, &EmptyMsg {})
            .unwrap();
        assert_eq!(2, qres.count);
    }
}
