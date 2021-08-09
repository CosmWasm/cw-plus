use std::fmt;

#[cfg(test)]
use cosmwasm_std::testing::{mock_env, MockApi};
use cosmwasm_std::{
    from_slice, to_vec, Addr, Api, Binary, BlockInfo, Coin, ContractResult, CosmosMsg, Empty,
    Querier, QuerierResult, QuerierWrapper, QueryRequest, Storage, SystemError, SystemResult,
};
use schemars::JsonSchema;
use serde::Serialize;

use crate::bank::Bank;
use crate::contracts::Contract;
use crate::executor::{AppResponse, Executor};
use crate::transactions::transactional;
use crate::wasm::{ContractData, Wasm, WasmKeeper};

pub fn next_block(block: &mut BlockInfo) {
    block.time = block.time.plus_seconds(5);
    block.height += 1;
}

/// Router is a persisted state. You can query this.
/// Execution generally happens on the RouterCache, which then can be atomically committed or rolled back.
/// We offer .execute() as a wrapper around cache, execute, commit/rollback process.
///
/// C is the custom message returned init, handle, sudo (Response<C>).
/// All contracts must return Response<C> or Response<Empty>
pub struct App<C = Empty>
where
    C: Clone + fmt::Debug + PartialEq + JsonSchema + 'static,
{
    router: Router<C>,
    api: Box<dyn Api>,
    storage: Box<dyn Storage>,
    block: BlockInfo,
}

impl<C> Querier for App<C>
where
    C: Clone + fmt::Debug + PartialEq + JsonSchema + 'static,
{
    fn raw_query(&self, bin_request: &[u8]) -> QuerierResult {
        self.router
            .querier(&*self.api, &*self.storage, &self.block)
            .raw_query(bin_request)
    }
}

impl<C> Executor<C> for App<C>
where
    C: Clone + fmt::Debug + PartialEq + JsonSchema + 'static,
{
    fn execute(&mut self, sender: Addr, msg: CosmosMsg<C>) -> Result<AppResponse, String> {
        let mut all = self.execute_multi(sender, vec![msg])?;
        let res = all.pop().unwrap();
        Ok(res)
    }
}

impl<C> App<C>
where
    C: Clone + fmt::Debug + PartialEq + JsonSchema + 'static,
{
    pub fn new(
        api: impl Api + 'static,
        block: BlockInfo,
        bank: impl Bank + 'static,
        storage: impl Storage + 'static,
    ) -> Self {
        App {
            router: Router::new(bank),
            api: Box::new(api),
            storage: Box::new(storage),
            block,
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

    /// Simple helper so we get access to all the QuerierWrapper helpers,
    /// eg. wrap().query_wasm_smart, query_all_balances, ...
    pub fn wrap(&self) -> QuerierWrapper {
        QuerierWrapper::new(self)
    }

    /// Runs multiple CosmosMsg in one atomic operation.
    /// This will create a cache before the execution, so no state changes are persisted if any of them
    /// return an error. But all writes are persisted on success.
    pub fn execute_multi(
        &mut self,
        sender: Addr,
        msgs: Vec<CosmosMsg<C>>,
    ) -> Result<Vec<AppResponse>, String> {
        // we need to do some caching of storage here, once in the entry point:
        // meaning, wrap current state, all writes go to a cache, only when execute
        // returns a success do we flush it (otherwise drop it)

        let Self {
            block,
            router,
            api,
            storage,
        } = self;

        transactional(&mut **storage, |write_cache, _| {
            msgs.into_iter()
                .map(|msg| router.execute(&**api, write_cache, block, sender.clone(), msg))
                .collect()
        })
    }

    /// This is an "admin" function to let us adjust bank accounts
    pub fn init_bank_balance(&mut self, account: &Addr, amount: Vec<Coin>) -> Result<(), String> {
        self.router
            .bank
            .init_balance(&mut *self.storage, account, amount)
    }

    /// This registers contract code (like uploading wasm bytecode on a chain),
    /// so it can later be used to instantiate a contract.
    pub fn store_code(&mut self, code: Box<dyn Contract<C>>) -> u64 {
        self.router.wasm.store_code(code) as u64
    }

    /// This allows to get `ContractData` for specific contract
    pub fn contract_data(&self, address: &Addr) -> Result<ContractData, String> {
        self.router.wasm.contract_data(&*self.storage, address)
    }

    /// Runs arbitrary CosmosMsg in "sudo" mode.
    /// This will create a cache before the execution, so no state changes are persisted if this
    /// returns an error, but all are persisted on success.
    pub fn sudo<T: Serialize, U: Into<Addr>>(
        &mut self,
        contract_addr: U,
        msg: &T,
    ) -> Result<AppResponse, String> {
        let msg = to_vec(msg).map_err(|e| e.to_string())?;
        self.router.wasm.sudo(
            &*self.api,
            contract_addr.into(),
            &mut *self.storage,
            &self.router,
            &self.block,
            msg,
        )
    }
}

pub struct Router<C> {
    pub wasm: Box<dyn Wasm<C>>,
    pub bank: Box<dyn Bank>,
}

impl<C> Router<C>
where
    C: Clone + fmt::Debug + PartialEq + JsonSchema + 'static,
{
    pub(super) fn new(bank: impl Bank + 'static) -> Self {
        Router {
            wasm: Box::new(WasmKeeper::new()),
            bank: Box::new(bank),
        }
    }

    pub fn querier<'a>(
        &'a self,
        api: &'a dyn Api,
        storage: &'a dyn Storage,
        block_info: &'a BlockInfo,
    ) -> RouterQuerier<'a, C> {
        RouterQuerier {
            router: self,
            api,
            storage,
            block_info,
        }
    }

    /// this is used by `RouterQuerier` to actual implement the `Querier` interface.
    /// you most likely want to use `router.querier(storage, block).wrap()` to get a
    /// QuerierWrapper to interact with
    pub fn query(
        &self,
        api: &dyn Api,
        storage: &dyn Storage,
        block: &BlockInfo,
        request: QueryRequest<Empty>,
    ) -> Result<Binary, String> {
        match request {
            QueryRequest::Wasm(req) => {
                self.wasm
                    .query(api, storage, &self.querier(api, storage, block), block, req)
            }
            QueryRequest::Bank(req) => self.bank.query(api, storage, req),
            _ => unimplemented!(),
        }
    }

    pub fn execute(
        &self,
        api: &dyn Api,
        storage: &mut dyn Storage,
        block: &BlockInfo,
        sender: Addr,
        msg: CosmosMsg<C>,
    ) -> Result<AppResponse, String> {
        match msg {
            CosmosMsg::Wasm(msg) => self.wasm.execute(api, storage, &self, block, sender, msg),
            CosmosMsg::Bank(msg) => self.bank.execute(storage, sender, msg),
            _ => unimplemented!(),
        }
    }
}

pub struct RouterQuerier<'a, C>
where
    C: Clone + fmt::Debug + PartialEq + JsonSchema + 'static,
{
    router: &'a Router<C>,
    api: &'a dyn Api,
    storage: &'a dyn Storage,
    block_info: &'a BlockInfo,
}
impl<'a, C> RouterQuerier<'a, C>
where
    C: Clone + fmt::Debug + PartialEq + JsonSchema + 'static,
{
    pub fn new(
        router: &'a Router<C>,
        api: &'a dyn Api,
        storage: &'a dyn Storage,
        block_info: &'a BlockInfo,
    ) -> Self {
        Self {
            router,
            api,
            storage,
            block_info,
        }
    }
}

impl<'a, C> Querier for RouterQuerier<'a, C>
where
    C: Clone + fmt::Debug + PartialEq + JsonSchema + 'static,
{
    fn raw_query(&self, bin_request: &[u8]) -> QuerierResult {
        // TODO: we need to make a new custom type for queries
        let request: QueryRequest<Empty> = match from_slice(bin_request) {
            Ok(v) => v,
            Err(e) => {
                return SystemResult::Err(SystemError::InvalidRequest {
                    error: format!("Parsing query request: {}", e.to_string()),
                    request: bin_request.into(),
                })
            }
        };
        let contract_result: ContractResult<Binary> = self
            .router
            .query(self.api, self.storage, self.block_info, request)
            .into();
        SystemResult::Ok(contract_result)
    }
}

#[cfg(test)]
mod test {
    use cosmwasm_std::testing::MockStorage;
    use cosmwasm_std::{
        attr, coin, coins, to_binary, AllBalanceResponse, Attribute, BankMsg, BankQuery, Event,
        Reply, StdResult, SubMsg, WasmMsg,
    };

    use crate::test_helpers::contracts::{echo, hackatom, payout, reflect};
    use crate::test_helpers::{CustomMsg, EmptyMsg};
    use crate::transactions::StorageTransaction;
    use crate::BankKeeper;

    use super::*;

    fn mock_app() -> App<Empty> {
        let env = mock_env();
        let api = MockApi::default();
        let bank = BankKeeper::new();

        App::new(api, env.block, bank, MockStorage::new())
    }

    fn custom_app() -> App<CustomMsg> {
        let env = mock_env();
        let api = MockApi::default();
        let bank = BankKeeper::new();

        App::new(api, env.block, bank, MockStorage::new())
    }

    fn get_balance<C>(app: &App<C>, addr: &Addr) -> Vec<Coin>
    where
        C: Clone + fmt::Debug + PartialEq + JsonSchema,
    {
        app.wrap().query_all_balances(addr).unwrap()
    }

    #[test]
    fn update_block() {
        let mut app = mock_app();

        let BlockInfo { time, height, .. } = app.block;
        app.update_block(next_block);

        assert_eq!(time.plus_seconds(5), app.block.time);
        assert_eq!(height + 1, app.block.height);
    }

    #[test]
    fn send_tokens() {
        let mut app = mock_app();

        let owner = Addr::unchecked("owner");
        let rcpt = Addr::unchecked("receiver");
        let init_funds = vec![coin(20, "btc"), coin(100, "eth")];
        let rcpt_funds = vec![coin(5, "btc")];

        // set money
        app.init_bank_balance(&owner, init_funds).unwrap();
        app.init_bank_balance(&rcpt, rcpt_funds).unwrap();

        // send both tokens
        let to_send = vec![coin(30, "eth"), coin(5, "btc")];
        let msg: CosmosMsg = BankMsg::Send {
            to_address: rcpt.clone().into(),
            amount: to_send,
        }
        .into();
        app.execute(owner.clone(), msg.clone()).unwrap();
        let rich = get_balance(&app, &owner);
        assert_eq!(vec![coin(15, "btc"), coin(70, "eth")], rich);
        let poor = get_balance(&app, &rcpt);
        assert_eq!(vec![coin(10, "btc"), coin(30, "eth")], poor);

        // can send from other account (but funds will be deducted from sender)
        app.execute(rcpt.clone(), msg).unwrap();

        // cannot send too much
        let msg = BankMsg::Send {
            to_address: rcpt.into(),
            amount: coins(20, "btc"),
        }
        .into();
        app.execute(owner.clone(), msg).unwrap_err();

        let rich = get_balance(&app, &owner);
        assert_eq!(vec![coin(15, "btc"), coin(70, "eth")], rich);
    }

    #[test]
    fn simple_contract() {
        let mut app = mock_app();

        // set personal balance
        let owner = Addr::unchecked("owner");
        let init_funds = vec![coin(20, "btc"), coin(100, "eth")];
        app.init_bank_balance(&owner, init_funds).unwrap();

        // set up contract
        let code_id = app.store_code(payout::contract());
        let msg = payout::InstantiateMessage {
            payout: coin(5, "eth"),
        };
        let contract_addr = app
            .instantiate_contract(
                code_id,
                owner.clone(),
                &msg,
                &coins(23, "eth"),
                "Payout",
                None,
            )
            .unwrap();

        let contract_data = app.contract_data(&contract_addr).unwrap();
        assert_eq!(
            contract_data,
            ContractData {
                code_id: code_id as usize,
                creator: owner.clone(),
                admin: None,
                label: "Payout".to_owned(),
                created: app.block_info().height
            }
        );

        // sender funds deducted
        let sender = get_balance(&app, &owner);
        assert_eq!(sender, vec![coin(20, "btc"), coin(77, "eth")]);
        // get contract address, has funds
        let funds = get_balance(&app, &contract_addr);
        assert_eq!(funds, coins(23, "eth"));

        // create empty account
        let random = Addr::unchecked("random");
        let funds = get_balance(&app, &random);
        assert_eq!(funds, vec![]);

        // do one payout and see money coming in
        let res = app
            .execute_contract(random.clone(), contract_addr.clone(), &EmptyMsg {}, &[])
            .unwrap();
        assert_eq!(2, res.events.len());
        let custom_attrs = res.custom_attrs(0);
        assert_eq!(&[attr("action", "payout")], &custom_attrs);
        let expected_transfer = Event::new("transfer")
            .add_attribute("recipient", "random")
            .add_attribute("sender", &contract_addr)
            .add_attribute("amount", "5eth");
        assert_eq!(&expected_transfer, &res.events[1]);

        // random got cash
        let funds = get_balance(&app, &random);
        assert_eq!(funds, coins(5, "eth"));
        // contract lost it
        let funds = get_balance(&app, &contract_addr);
        assert_eq!(funds, coins(18, "eth"));
    }

    #[test]
    fn reflect_success() {
        let mut app = custom_app();

        // set personal balance
        let owner = Addr::unchecked("owner");
        let init_funds = vec![coin(20, "btc"), coin(100, "eth")];
        app.init_bank_balance(&owner, init_funds).unwrap();

        // set up payout contract
        let payout_id = app.store_code(payout::contract());
        let msg = payout::InstantiateMessage {
            payout: coin(5, "eth"),
        };
        let payout_addr = app
            .instantiate_contract(
                payout_id,
                owner.clone(),
                &msg,
                &coins(23, "eth"),
                "Payout",
                None,
            )
            .unwrap();

        // set up reflect contract
        let reflect_id = app.store_code(reflect::contract());
        let reflect_addr = app
            .instantiate_contract(reflect_id, owner, &EmptyMsg {}, &[], "Reflect", None)
            .unwrap();

        // reflect account is empty
        let funds = get_balance(&app, &reflect_addr);
        assert_eq!(funds, vec![]);
        // reflect count is 1
        let qres: payout::CountResponse = app
            .wrap()
            .query_wasm_smart(&reflect_addr, &reflect::QueryMsg::Count {})
            .unwrap();
        assert_eq!(0, qres.count);

        // reflecting payout message pays reflect contract
        let msg = SubMsg::new(WasmMsg::Execute {
            contract_addr: payout_addr.clone().into(),
            msg: b"{}".into(),
            funds: vec![],
        });
        let msgs = reflect::Message {
            messages: vec![msg],
        };
        let res = app
            .execute_contract(Addr::unchecked("random"), reflect_addr.clone(), &msgs, &[])
            .unwrap();

        // ensure the attributes were relayed from the sub-message
        assert_eq!(2, res.events.len(), "{:?}", res.events);
        // call to reflect returns no event + attributes, nothing emitted
        // the call to payout does emit an event, which is returned
        let first = &res.events[0];
        assert_eq!(first.ty.as_str(), "wasm");
        assert_eq!(2, first.attributes.len());
        assert_eq!(
            &attr("contract_address", &payout_addr),
            &first.attributes[0]
        );
        assert_eq!(&attr("action", "payout"), &first.attributes[1]);
        // third event is the transfer from bank
        let second = &res.events[1];
        assert_eq!(second.ty.as_str(), "transfer");

        // ensure transfer was executed with reflect as sender
        let funds = get_balance(&app, &reflect_addr);
        assert_eq!(funds, coins(5, "eth"));

        // reflect count updated
        let qres: payout::CountResponse = app
            .wrap()
            .query_wasm_smart(&reflect_addr, &reflect::QueryMsg::Count {})
            .unwrap();
        assert_eq!(1, qres.count);
    }

    #[test]
    fn reflect_error() {
        let mut app = custom_app();

        // set personal balance
        let owner = Addr::unchecked("owner");
        let init_funds = vec![coin(20, "btc"), coin(100, "eth")];
        app.init_bank_balance(&owner, init_funds).unwrap();

        // set up reflect contract
        let reflect_id = app.store_code(reflect::contract());
        let reflect_addr = app
            .instantiate_contract(
                reflect_id,
                owner,
                &EmptyMsg {},
                &coins(40, "eth"),
                "Reflect",
                None,
            )
            .unwrap();

        // reflect has 40 eth
        let funds = get_balance(&app, &reflect_addr);
        assert_eq!(funds, coins(40, "eth"));
        let random = Addr::unchecked("random");

        // sending 7 eth works
        let msg = SubMsg::new(BankMsg::Send {
            to_address: random.clone().into(),
            amount: coins(7, "eth"),
        });
        let msgs = reflect::Message {
            messages: vec![msg],
        };
        let res = app
            .execute_contract(random.clone(), reflect_addr.clone(), &msgs, &[])
            .unwrap();
        // no wasm events as no attributes
        assert_eq!(1, res.events.len());
        // only transfer event from bank
        let transfer = &res.events[0];
        assert_eq!(transfer.ty.as_str(), "transfer");

        // ensure random got paid
        let funds = get_balance(&app, &random);
        assert_eq!(funds, coins(7, "eth"));

        // reflect count should be updated to 1
        let qres: payout::CountResponse = app
            .wrap()
            .query_wasm_smart(&reflect_addr, &reflect::QueryMsg::Count {})
            .unwrap();
        assert_eq!(1, qres.count);

        // sending 8 eth, then 3 btc should fail both
        let msg = SubMsg::new(BankMsg::Send {
            to_address: random.clone().into(),
            amount: coins(8, "eth"),
        });
        let msg2 = SubMsg::new(BankMsg::Send {
            to_address: random.clone().into(),
            amount: coins(3, "btc"),
        });
        let msgs = reflect::Message {
            messages: vec![msg, msg2],
        };
        let err = app
            .execute_contract(random.clone(), reflect_addr.clone(), &msgs, &[])
            .unwrap_err();
        assert_eq!("Overflow: Cannot Sub with 0 and 3", err.as_str());

        // first one should have been rolled-back on error (no second payment)
        let funds = get_balance(&app, &random);
        assert_eq!(funds, coins(7, "eth"));

        // failure should not update reflect count
        let qres: payout::CountResponse = app
            .wrap()
            .query_wasm_smart(&reflect_addr, &reflect::QueryMsg::Count {})
            .unwrap();
        assert_eq!(1, qres.count);
    }

    #[test]
    fn sudo_works() {
        let mut app = mock_app();

        let owner = Addr::unchecked("owner");
        let init_funds = vec![coin(100, "eth")];
        app.init_bank_balance(&owner, init_funds).unwrap();
        let payout_id = app.store_code(payout::contract());
        let msg = payout::InstantiateMessage {
            payout: coin(5, "eth"),
        };
        let payout_addr = app
            .instantiate_contract(payout_id, owner, &msg, &coins(23, "eth"), "Payout", None)
            .unwrap();

        // count is 1
        let payout::CountResponse { count } = app
            .wrap()
            .query_wasm_smart(&payout_addr, &payout::QueryMsg::Count {})
            .unwrap();
        assert_eq!(1, count);

        // sudo
        let msg = payout::SudoMsg { set_count: 25 };
        app.sudo(payout_addr.clone(), &msg).unwrap();

        // count is 25
        let payout::CountResponse { count } = app
            .wrap()
            .query_wasm_smart(&payout_addr, &payout::QueryMsg::Count {})
            .unwrap();
        assert_eq!(25, count);
    }

    #[test]
    fn reflect_submessage_reply_works() {
        let mut app = custom_app();

        // set personal balance
        let owner = Addr::unchecked("owner");
        let random = Addr::unchecked("random");
        let init_funds = vec![coin(20, "btc"), coin(100, "eth")];
        app.init_bank_balance(&owner, init_funds).unwrap();

        // set up reflect contract
        let reflect_id = app.store_code(reflect::contract());
        let reflect_addr = app
            .instantiate_contract(
                reflect_id,
                owner,
                &EmptyMsg {},
                &coins(40, "eth"),
                "Reflect",
                None,
            )
            .unwrap();

        // no reply writen beforehand
        let query = reflect::QueryMsg::Reply { id: 123 };
        let res: StdResult<Reply> = app.wrap().query_wasm_smart(&reflect_addr, &query);
        res.unwrap_err();

        // reflect sends 7 eth, success
        let msg = SubMsg::reply_always(
            BankMsg::Send {
                to_address: random.clone().into(),
                amount: coins(7, "eth"),
            },
            123,
        );
        let msgs = reflect::Message {
            messages: vec![msg],
        };
        let res = app
            .execute_contract(random.clone(), reflect_addr.clone(), &msgs, &[])
            .unwrap();
        // we should get 2 events, the transfer event and the custom event
        assert_eq!(2, res.events.len(), "{:?}", res.events);
        // first event is the transfer from bank
        let transfer = &res.events[0];
        assert_eq!(transfer.ty.as_str(), "transfer");
        // the second one is a custom event (from reply)
        let custom = &res.events[1];
        assert_eq!("wasm-custom", custom.ty.as_str());
        assert_eq!(2, custom.attributes.len());
        assert_eq!(&attr("from", "reply"), &custom.attributes[0]);
        assert_eq!(&attr("to", "test"), &custom.attributes[1]);

        // ensure success was written
        let res: Reply = app.wrap().query_wasm_smart(&reflect_addr, &query).unwrap();
        assert_eq!(res.id, 123);
        assert!(res.result.is_ok());

        // reflect sends 300 btc, failure, but error caught by submessage (so shows success)
        let msg = SubMsg::reply_always(
            BankMsg::Send {
                to_address: random.clone().into(),
                amount: coins(300, "btc"),
            },
            456,
        );
        let msgs = reflect::Message {
            messages: vec![msg],
        };
        let _res = app
            .execute_contract(random, reflect_addr.clone(), &msgs, &[])
            .unwrap();

        // ensure error was written
        let query = reflect::QueryMsg::Reply { id: 456 };
        let res: Reply = app.wrap().query_wasm_smart(&reflect_addr, &query).unwrap();
        assert_eq!(res.id, 456);
        assert!(res.result.is_err());
        // TODO: check error?
    }

    fn query_router<C>(
        router: &Router<C>,
        api: &dyn Api,
        storage: &dyn Storage,
        rcpt: &Addr,
    ) -> Vec<Coin>
    where
        C: Clone + fmt::Debug + PartialEq + JsonSchema,
    {
        let query = BankQuery::AllBalances {
            address: rcpt.into(),
        };
        let res = router.bank.query(api, storage, query).unwrap();
        let val: AllBalanceResponse = from_slice(&res).unwrap();
        val.amount
    }

    fn query_app<C>(app: &App<C>, rcpt: &Addr) -> Vec<Coin>
    where
        C: Clone + fmt::Debug + PartialEq + JsonSchema,
    {
        let query = BankQuery::AllBalances {
            address: rcpt.into(),
        }
        .into();
        let val: AllBalanceResponse = app.wrap().query(&query).unwrap();
        val.amount
    }

    #[test]
    fn multi_level_bank_cache() {
        let mut app = mock_app();

        // set personal balance
        let owner = Addr::unchecked("owner");
        let rcpt = Addr::unchecked("recipient");
        let init_funds = vec![coin(20, "btc"), coin(100, "eth")];
        app.init_bank_balance(&owner, init_funds).unwrap();

        // cache 1 - send some tokens
        let mut cache = StorageTransaction::new(&*app.storage);
        let msg = BankMsg::Send {
            to_address: rcpt.clone().into(),
            amount: coins(25, "eth"),
        };
        app.router
            .execute(&*app.api, &mut cache, &app.block, owner.clone(), msg.into())
            .unwrap();

        // shows up in cache
        let cached_rcpt = query_router(&app.router, &*app.api, &cache, &rcpt);
        assert_eq!(coins(25, "eth"), cached_rcpt);
        let router_rcpt = query_app(&app, &rcpt);
        assert_eq!(router_rcpt, vec![]);

        // now, second level cache
        transactional(&mut cache, |cache2, read| {
            let msg = BankMsg::Send {
                to_address: rcpt.clone().into(),
                amount: coins(12, "eth"),
            };
            app.router
                .execute(&*app.api, cache2, &app.block, owner, msg.into())
                .unwrap();

            // shows up in 2nd cache
            let cached_rcpt = query_router(&app.router, &*app.api, read, &rcpt);
            assert_eq!(coins(25, "eth"), cached_rcpt);
            let cached2_rcpt = query_router(&app.router, &*app.api, cache2, &rcpt);
            assert_eq!(coins(37, "eth"), cached2_rcpt);
            Ok(())
        })
        .unwrap();

        // apply first to router
        cache.prepare().commit(&mut *app.storage);

        let committed = query_app(&app, &rcpt);
        assert_eq!(coins(37, "eth"), committed);
    }

    #[test]
    fn sent_funds_properly_visible_on_execution() {
        // Testing if funds on contract are properly visible on contract.
        // Hackatom contract is initialized with 10btc. Then, the contract is executed, with
        // additional 20btc. Then beneficiary balance is checked - expeced value is 30btc. 10btc
        // would mean that sending tokens with message is not visible for this very message, and
        // 20btc means, that only such just send funds are visible.
        let mut app = mock_app();

        let owner = Addr::unchecked("owner");
        let beneficiary = Addr::unchecked("beneficiary");
        app.init_bank_balance(&owner, coins(30, "btc")).unwrap();

        let contract_id = app.store_code(hackatom::contract());
        let contract = app
            .instantiate_contract(
                contract_id,
                owner.clone(),
                &hackatom::InstantiateMsg {
                    beneficiary: beneficiary.as_str().to_owned(),
                },
                &coins(10, "btc"),
                "Hackatom",
                None,
            )
            .unwrap();

        app.execute_contract(
            owner.clone(),
            contract.clone(),
            &EmptyMsg {},
            &coins(20, "btc"),
        )
        .unwrap();

        // Check balance of all accounts to ensure no tokens where burned or created, and they are
        // in correct places
        assert_eq!(get_balance(&app, &owner), &[]);
        assert_eq!(get_balance(&app, &contract), &[]);
        assert_eq!(get_balance(&app, &beneficiary), coins(30, "btc"));
    }

    #[test]
    fn sent_wasm_migration_works() {
        // The plan:
        // create a hackatom contract with some funds
        // check admin set properly
        // check beneficiary set properly
        // migrate fails if not admin
        // migrate succeeds if admin
        // check beneficiary updated
        let mut app = mock_app();

        let owner = Addr::unchecked("owner");
        let beneficiary = Addr::unchecked("beneficiary");
        app.init_bank_balance(&owner, coins(30, "btc")).unwrap();

        // create a hackatom contract with some funds
        let contract_id = app.store_code(hackatom::contract());
        let contract = app
            .instantiate_contract(
                contract_id,
                owner.clone(),
                &hackatom::InstantiateMsg {
                    beneficiary: beneficiary.as_str().to_owned(),
                },
                &coins(20, "btc"),
                "Hackatom",
                Some(owner.to_string()),
            )
            .unwrap();

        // check admin set properly
        let info = app.contract_data(&contract).unwrap();
        assert_eq!(info.admin, Some(owner.clone()));
        // check beneficiary set properly
        let state: hackatom::InstantiateMsg = app
            .wrap()
            .query_wasm_smart(&contract, &hackatom::QueryMsg::Beneficiary {})
            .unwrap();
        assert_eq!(state.beneficiary, beneficiary);

        // migrate fails if not admin
        let random = Addr::unchecked("random");
        let migrate_msg = hackatom::MigrateMsg {
            new_guy: random.to_string(),
        };
        app.migrate_contract(beneficiary, contract.clone(), &migrate_msg, contract_id)
            .unwrap_err();

        // migrate fails if unregistred code id
        app.migrate_contract(
            owner.clone(),
            contract.clone(),
            &migrate_msg,
            contract_id + 7,
        )
        .unwrap_err();

        // migrate succeeds when the stars align
        app.migrate_contract(owner, contract.clone(), &migrate_msg, contract_id)
            .unwrap();

        // check beneficiary updated
        let state: hackatom::InstantiateMsg = app
            .wrap()
            .query_wasm_smart(&contract, &hackatom::QueryMsg::Beneficiary {})
            .unwrap();
        assert_eq!(state.beneficiary, random);
    }

    mod replay_data_overwrite {
        use super::*;

        fn make_echo_submsg(
            contract: Addr,
            data: impl Into<Option<&'static str>>,
            sub_msg: Vec<SubMsg>,
        ) -> SubMsg {
            let data = data.into().map(|s| s.to_owned());
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: contract.into(),
                msg: to_binary(&echo::Message {
                    data,
                    sub_msg,
                    ..echo::Message::default()
                })
                .unwrap(),
                funds: vec![],
            }))
        }

        #[test]
        fn no_submsg() {
            let mut app = mock_app();

            let owner = Addr::unchecked("owner");

            let contract_id = app.store_code(echo::contract());
            let contract = app
                .instantiate_contract(contract_id, owner.clone(), &EmptyMsg {}, &[], "Echo", None)
                .unwrap();

            let response = app
                .execute_contract(
                    owner,
                    contract,
                    &echo::Message {
                        data: Some("Data".to_owned()),
                        ..echo::Message::default()
                    },
                    &[],
                )
                .unwrap();

            assert_eq!(response.data, Some("Data".as_bytes().into()));
        }

        #[test]
        fn single_submsg() {
            let mut app = mock_app();

            let owner = Addr::unchecked("owner");

            let contract_id = app.store_code(echo::contract());
            let contract = app
                .instantiate_contract(contract_id, owner.clone(), &EmptyMsg {}, &[], "Echo", None)
                .unwrap();

            let response = app
                .execute_contract(
                    owner,
                    contract.clone(),
                    &echo::Message {
                        data: Some("First".to_owned()),
                        sub_msg: vec![make_echo_submsg(contract, "Second", vec![])],
                        ..echo::Message::default()
                    },
                    &[],
                )
                .unwrap();

            assert_eq!(response.data, Some("Second".as_bytes().into()));
        }

        #[test]
        fn single_no_data() {
            let mut app = mock_app();

            let owner = Addr::unchecked("owner");

            let contract_id = app.store_code(echo::contract());
            let contract = app
                .instantiate_contract(contract_id, owner.clone(), &EmptyMsg {}, &[], "Echo", None)
                .unwrap();

            let response = app
                .execute_contract(
                    owner,
                    contract.clone(),
                    &echo::Message {
                        data: Some("First".to_owned()),
                        sub_msg: vec![make_echo_submsg(contract, None, vec![])],
                        ..echo::Message::default()
                    },
                    &[],
                )
                .unwrap();

            assert_eq!(response.data, Some("First".as_bytes().into()));
        }

        #[test]
        fn multiple_submsg() {
            let mut app = mock_app();

            let owner = Addr::unchecked("owner");

            let contract_id = app.store_code(echo::contract());
            let contract = app
                .instantiate_contract(contract_id, owner.clone(), &EmptyMsg {}, &[], "Echo", None)
                .unwrap();

            let response = app
                .execute_contract(
                    owner,
                    contract.clone(),
                    &echo::Message {
                        data: Some("Orig".to_owned()),
                        sub_msg: vec![
                            make_echo_submsg(contract.clone(), None, vec![]),
                            make_echo_submsg(contract.clone(), "First", vec![]),
                            make_echo_submsg(contract.clone(), "Second", vec![]),
                            make_echo_submsg(contract, None, vec![]),
                        ],
                        ..echo::Message::default()
                    },
                    &[],
                )
                .unwrap();

            assert_eq!(response.data, Some("Second".as_bytes().into()));
        }

        #[test]
        fn nested_submsg() {
            let mut app = mock_app();

            let owner = Addr::unchecked("owner");

            let contract_id = app.store_code(echo::contract());
            let contract = app
                .instantiate_contract(contract_id, owner.clone(), &EmptyMsg {}, &[], "Echo", None)
                .unwrap();

            let response = app
                .execute_contract(
                    owner,
                    contract.clone(),
                    &echo::Message {
                        data: Some("Orig".to_owned()),
                        sub_msg: vec![make_echo_submsg(
                            contract.clone(),
                            None,
                            vec![make_echo_submsg(
                                contract.clone(),
                                "First",
                                vec![make_echo_submsg(
                                    contract.clone(),
                                    "Second",
                                    vec![make_echo_submsg(contract, None, vec![])],
                                )],
                            )],
                        )],
                        ..echo::Message::default()
                    },
                    &[],
                )
                .unwrap();

            assert_eq!(response.data, Some("Second".as_bytes().into()));
        }
    }

    mod response_validation {
        use super::*;

        #[test]
        fn empty_attribute_key() {
            let mut app = mock_app();

            let owner = Addr::unchecked("owner");

            let contract_id = app.store_code(echo::contract());
            let contract = app
                .instantiate_contract(contract_id, owner.clone(), &EmptyMsg {}, &[], "Echo", None)
                .unwrap();

            let err = app
                .execute_contract(
                    owner,
                    contract,
                    &echo::Message {
                        data: None,
                        attributes: vec![
                            Attribute::new("   ", "value"),
                            Attribute::new("proper", "proper_val"),
                        ],
                        ..echo::Message::default()
                    },
                    &[],
                )
                .unwrap_err();

            assert_eq!(err, "Empty attribute key. Value: value");
        }

        #[test]
        fn empty_attribute_value() {
            let mut app = mock_app();

            let owner = Addr::unchecked("owner");

            let contract_id = app.store_code(echo::contract());
            let contract = app
                .instantiate_contract(contract_id, owner.clone(), &EmptyMsg {}, &[], "Echo", None)
                .unwrap();

            let err = app
                .execute_contract(
                    owner,
                    contract,
                    &echo::Message {
                        data: None,
                        attributes: vec![
                            Attribute::new("key", "   "),
                            Attribute::new("proper", "proper_val"),
                        ],
                        ..echo::Message::default()
                    },
                    &[],
                )
                .unwrap_err();

            assert_eq!(err, "Empty attribute value. Key: key");
        }

        #[test]
        fn empty_event_attribute_key() {
            let mut app = mock_app();

            let owner = Addr::unchecked("owner");

            let contract_id = app.store_code(echo::contract());
            let contract = app
                .instantiate_contract(contract_id, owner.clone(), &EmptyMsg {}, &[], "Echo", None)
                .unwrap();

            let err = app
                .execute_contract(
                    owner,
                    contract,
                    &echo::Message {
                        data: None,
                        events: vec![Event::new("event")
                            .add_attribute("   ", "value")
                            .add_attribute("proper", "proper_val")],
                        ..echo::Message::default()
                    },
                    &[],
                )
                .unwrap_err();

            assert_eq!(err, "Empty attribute key. Value: value");
        }

        #[test]
        fn empty_event_attribute_value() {
            let mut app = mock_app();

            let owner = Addr::unchecked("owner");

            let contract_id = app.store_code(echo::contract());
            let contract = app
                .instantiate_contract(contract_id, owner.clone(), &EmptyMsg {}, &[], "Echo", None)
                .unwrap();

            let err = app
                .execute_contract(
                    owner,
                    contract,
                    &echo::Message {
                        data: None,
                        events: vec![Event::new("event")
                            .add_attribute("key", "   ")
                            .add_attribute("proper", "proper_val")],
                        ..echo::Message::default()
                    },
                    &[],
                )
                .unwrap_err();

            assert_eq!(err, "Empty attribute value. Key: key");
        }

        #[test]
        fn too_short_event_type() {
            let mut app = mock_app();

            let owner = Addr::unchecked("owner");

            let contract_id = app.store_code(echo::contract());
            let contract = app
                .instantiate_contract(contract_id, owner.clone(), &EmptyMsg {}, &[], "Echo", None)
                .unwrap();

            let err = app
                .execute_contract(
                    owner,
                    contract,
                    &echo::Message {
                        data: None,
                        events: vec![Event::new(" e "), Event::new("event")],
                        ..echo::Message::default()
                    },
                    &[],
                )
                .unwrap_err();

            assert_eq!(err, "Event type too short: e");
        }
    }
}
