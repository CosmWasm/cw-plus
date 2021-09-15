use std::fmt::{self, Debug};

use cosmwasm_std::testing::{mock_env, MockApi, MockStorage};
use cosmwasm_std::{
    from_slice, to_vec, Addr, Api, BankMsg, Binary, BlockInfo, Coin, ContractResult, CustomQuery,
    Empty, Querier, QuerierResult, QuerierWrapper, QueryRequest, Storage, SystemError,
    SystemResult, WasmMsg,
};
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::bank::Bank;
use crate::contracts::Contract;
use crate::custom_handler::{CustomHandler, PanickingCustomHandler};
use crate::executor::{AppResponse, Executor};
use crate::transactions::transactional;
use crate::wasm::{ContractData, Wasm, WasmKeeper};
use crate::BankKeeper;

use anyhow::Result as AnyResult;

pub fn next_block(block: &mut BlockInfo) {
    block.time = block.time.plus_seconds(5);
    block.height += 1;
}

/// Type alias for default build `App` to make its storing simpler in typical scenario
pub type BasicApp<ExecC = Empty, QueryC = Empty> = App<
    BankKeeper,
    MockApi,
    MockStorage,
    PanickingCustomHandler<ExecC, QueryC>,
    WasmKeeper<ExecC, QueryC>,
>;

/// Router is a persisted state. You can query this.
/// Execution generally happens on the RouterCache, which then can be atomically committed or rolled back.
/// We offer .execute() as a wrapper around cache, execute, commit/rollback process.
pub struct App<
    Bank = BankKeeper,
    Api = MockApi,
    Storage = MockStorage,
    Custom = PanickingCustomHandler<Empty, Empty>,
    Wasm = WasmKeeper<Empty, Empty>,
> {
    router: Router<Bank, Custom, Wasm>,
    api: Api,
    storage: Storage,
    block: BlockInfo,
}

impl Default for BasicApp {
    fn default() -> Self {
        Self::new()
    }
}

impl BasicApp {
    /// Creates new default `App` implementation working with Empty custom messages.
    pub fn new() -> Self {
        AppBuilder::new().build()
    }
}

/// Creates new default `App` implementation working with customized exec and query messages.
/// Outside of `App` implementation to make type elision better.
pub fn custom_app<ExecC, QueryC>() -> BasicApp<ExecC, QueryC>
where
    ExecC: Debug + Clone + PartialEq + JsonSchema + DeserializeOwned + 'static,
    QueryC: Debug + CustomQuery + DeserializeOwned + 'static,
{
    AppBuilder::new_custom().build()
}

impl<BankT, ApiT, StorageT, CustomT, WasmT> Querier for App<BankT, ApiT, StorageT, CustomT, WasmT>
where
    CustomT::ExecC: Clone + fmt::Debug + PartialEq + JsonSchema + DeserializeOwned + 'static,
    CustomT::QueryC: CustomQuery + DeserializeOwned + 'static,
    WasmT: Wasm<CustomT::ExecC, CustomT::QueryC>,
    BankT: Bank,
    ApiT: Api,
    StorageT: Storage,
    CustomT: CustomHandler,
{
    fn raw_query(&self, bin_request: &[u8]) -> QuerierResult {
        self.router
            .querier(&self.api, &self.storage, &self.block)
            .raw_query(bin_request)
    }
}

impl<BankT, ApiT, StorageT, CustomT, WasmT> Executor<CustomT::ExecC>
    for App<BankT, ApiT, StorageT, CustomT, WasmT>
where
    CustomT::ExecC: Clone + fmt::Debug + PartialEq + JsonSchema + DeserializeOwned + 'static,
    CustomT::QueryC: CustomQuery + DeserializeOwned + 'static,
    WasmT: Wasm<CustomT::ExecC, CustomT::QueryC>,
    BankT: Bank,
    ApiT: Api,
    StorageT: Storage,
    CustomT: CustomHandler,
{
    fn execute(
        &mut self,
        sender: Addr,
        msg: cosmwasm_std::CosmosMsg<CustomT::ExecC>,
    ) -> AnyResult<AppResponse> {
        let mut all = self.execute_multi(sender, vec![msg])?;
        let res = all.pop().unwrap();
        Ok(res)
    }
}

/// Utility to build App in stages. If particular items wont be set, defaults would be used
pub struct AppBuilder<Bank, Api, Storage, Custom, Wasm> {
    wasm: Wasm,
    bank: Bank,
    api: Api,
    storage: Storage,
    custom: Custom,
    block: BlockInfo,
}

impl Default
    for AppBuilder<
        BankKeeper,
        MockApi,
        MockStorage,
        PanickingCustomHandler<Empty, Empty>,
        WasmKeeper<Empty, Empty>,
    >
{
    fn default() -> Self {
        Self::new()
    }
}

impl
    AppBuilder<
        BankKeeper,
        MockApi,
        MockStorage,
        PanickingCustomHandler<Empty, Empty>,
        WasmKeeper<Empty, Empty>,
    >
{
    /// Creates builder with default components working with empty exec and query messages.
    pub fn new() -> Self {
        AppBuilder {
            wasm: WasmKeeper::new(),
            bank: BankKeeper::new(),
            api: MockApi::default(),
            storage: MockStorage::new(),
            custom: PanickingCustomHandler::new(),
            block: mock_env().block,
        }
    }
}

impl<ExecC, QueryC>
    AppBuilder<
        BankKeeper,
        MockApi,
        MockStorage,
        PanickingCustomHandler<ExecC, QueryC>,
        WasmKeeper<ExecC, QueryC>,
    >
where
    ExecC: Debug + Clone + PartialEq + JsonSchema + DeserializeOwned + 'static,
    QueryC: Debug + CustomQuery + DeserializeOwned + 'static,
{
    /// Creates builder with default components designed to work with custom exec and query
    /// messages.
    pub fn new_custom() -> Self {
        AppBuilder {
            wasm: WasmKeeper::new(),
            bank: BankKeeper::new(),
            api: MockApi::default(),
            storage: MockStorage::new(),
            custom: PanickingCustomHandler::new(),
            block: mock_env().block,
        }
    }
}

impl<BankT, ApiT, StorageT, CustomT, WasmT> AppBuilder<BankT, ApiT, StorageT, CustomT, WasmT> {
    /// Overwrites default wasm executor.
    ///
    /// At this point it is needed that new wasm implements some `Wasm` trait, but it doesn't need
    /// to be bound to Bank or Custom yet - as those may change. The cross-components validation is
    /// done on final building.
    ///
    /// Also it is possible to completely abandon trait bounding here which would not be bad idea,
    /// however it might make the message on build creepy in many cases, so as for properly build
    /// `App` we always want `Wasm` to be `Wasm`, some checks are done early.
    pub fn with_wasm<B, C: CustomHandler, NewWasm: Wasm<C::ExecC, C::QueryC>>(
        self,
        wasm: NewWasm,
    ) -> AppBuilder<BankT, ApiT, StorageT, CustomT, NewWasm> {
        let AppBuilder {
            bank,
            api,
            storage,
            custom,
            block,
            ..
        } = self;

        AppBuilder {
            wasm,
            bank,
            api,
            storage,
            custom,
            block,
        }
    }

    /// Overwrites default bank interface
    pub fn with_bank<NewBank: Bank>(
        self,
        bank: NewBank,
    ) -> AppBuilder<NewBank, ApiT, StorageT, CustomT, WasmT> {
        let AppBuilder {
            wasm,
            api,
            storage,
            custom,
            block,
            ..
        } = self;

        AppBuilder {
            wasm,
            bank,
            api,
            storage,
            custom,
            block,
        }
    }

    /// Overwrites default api interface
    pub fn with_api<NewApi: Api>(
        self,
        api: NewApi,
    ) -> AppBuilder<BankT, NewApi, StorageT, CustomT, WasmT> {
        let AppBuilder {
            wasm,
            bank,
            storage,
            custom,
            block,
            ..
        } = self;

        AppBuilder {
            wasm,
            bank,
            api,
            storage,
            custom,
            block,
        }
    }

    /// Overwrites default storage interface
    pub fn with_storage<NewStorage: Storage>(
        self,
        storage: NewStorage,
    ) -> AppBuilder<BankT, ApiT, NewStorage, CustomT, WasmT> {
        let AppBuilder {
            wasm,
            api,
            bank,
            custom,
            block,
            ..
        } = self;

        AppBuilder {
            wasm,
            bank,
            api,
            storage,
            custom,
            block,
        }
    }

    /// Overwrites default custom messages handler
    ///
    /// At this point it is needed that new custom implements some `CustomHandler` trait, but it doesn't need
    /// to be bound to ExecC or QueryC yet - as those may change. The cross-components validation is
    /// done on final building.
    ///
    /// Also it is possible to completely abandon trait bounding here which would not be bad idea,
    /// however it might make the message on build creepy in many cases, so as for properly build
    /// `App` we always want `Wasm` to be `Wasm`, some checks are done early.
    pub fn with_custom<NewCustom: CustomHandler>(
        self,
        custom: NewCustom,
    ) -> AppBuilder<BankT, ApiT, StorageT, NewCustom, WasmT> {
        let AppBuilder {
            wasm,
            bank,
            api,
            storage,
            block,
            ..
        } = self;

        AppBuilder {
            wasm,
            bank,
            api,
            storage,
            custom,
            block,
        }
    }

    /// Overwrites default initial block
    pub fn with_block(mut self, block: BlockInfo) -> Self {
        self.block = block;
        self
    }

    /// Builds final `App`. At this point all components type have to be properly related to each
    /// other. If there are some generics related compilation error make sure, that all components
    /// are properly relating to each other.
    pub fn build(self) -> App<BankT, ApiT, StorageT, CustomT, WasmT>
    where
        BankT: Bank,
        ApiT: Api,
        StorageT: Storage,
        CustomT: CustomHandler,
        WasmT: Wasm<CustomT::ExecC, CustomT::QueryC>,
    {
        let router = Router {
            wasm: self.wasm,
            bank: self.bank,
            custom: self.custom,
        };

        App {
            router,
            api: self.api,
            storage: self.storage,
            block: self.block,
        }
    }
}

impl<BankT, ApiT, StorageT, CustomT, WasmT> App<BankT, ApiT, StorageT, CustomT, WasmT>
where
    CustomT::ExecC: std::fmt::Debug + PartialEq + Clone + JsonSchema + DeserializeOwned + 'static,
    CustomT::QueryC: CustomQuery + DeserializeOwned + 'static,
    WasmT: Wasm<CustomT::ExecC, CustomT::QueryC>,
    BankT: Bank,
    ApiT: Api,
    StorageT: Storage,
    CustomT: CustomHandler,
{
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
        msgs: Vec<cosmwasm_std::CosmosMsg<CustomT::ExecC>>,
    ) -> AnyResult<Vec<AppResponse>> {
        // we need to do some caching of storage here, once in the entry point:
        // meaning, wrap current state, all writes go to a cache, only when execute
        // returns a success do we flush it (otherwise drop it)

        let Self {
            block,
            router,
            api,
            storage,
        } = self;

        transactional(&mut *storage, |write_cache, _| {
            msgs.into_iter()
                .map(|msg| router.execute(&*api, write_cache, block, sender.clone(), msg.into()))
                .collect()
        })
    }

    /// This is an "admin" function to let us adjust bank accounts
    pub fn init_bank_balance(&mut self, account: &Addr, amount: Vec<Coin>) -> AnyResult<()> {
        self.router
            .bank
            .init_balance(&mut self.storage, account, amount)
    }

    /// This registers contract code (like uploading wasm bytecode on a chain),
    /// so it can later be used to instantiate a contract.
    pub fn store_code(&mut self, code: Box<dyn Contract<CustomT::ExecC>>) -> u64 {
        self.router.wasm.store_code(code) as u64
    }

    /// This allows to get `ContractData` for specific contract
    pub fn contract_data(&self, address: &Addr) -> AnyResult<ContractData> {
        self.router.wasm.contract_data(&self.storage, address)
    }

    /// Runs arbitrary CosmosMsg in "sudo" mode.
    /// This will create a cache before the execution, so no state changes are persisted if this
    /// returns an error, but all are persisted on success.
    pub fn sudo<T: Serialize, U: Into<Addr>>(
        &mut self,
        contract_addr: U,
        msg: &T,
    ) -> AnyResult<AppResponse> {
        let msg = to_vec(msg)?;
        self.router.wasm.sudo(
            &self.api,
            contract_addr.into(),
            &mut self.storage,
            &self.router,
            &self.block,
            msg,
        )
    }
}

pub struct Router<Bank, Custom, Wasm> {
    pub(crate) wasm: Wasm,
    pub(crate) bank: Bank,
    pub(crate) custom: Custom,
}

impl<BankT, CustomT, WasmT> Router<BankT, CustomT, WasmT>
where
    CustomT::ExecC: Clone + fmt::Debug + PartialEq + JsonSchema + DeserializeOwned + 'static,
    CustomT::QueryC: CustomQuery + DeserializeOwned + 'static,
    CustomT: CustomHandler,
    WasmT: Wasm<CustomT::ExecC, CustomT::QueryC>,
    BankT: Bank,
{
    pub fn querier<'a>(
        &'a self,
        api: &'a dyn Api,
        storage: &'a dyn Storage,
        block_info: &'a BlockInfo,
    ) -> RouterQuerier<'a, CustomT::ExecC, CustomT::QueryC> {
        RouterQuerier {
            router: self,
            api,
            storage,
            block_info,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
// See https://github.com/serde-rs/serde/issues/1296 why we cannot add De-Serialize trait bounds to T
pub enum CosmosMsg<T = Empty> {
    Bank(BankMsg),
    // by default we use RawMsg, but a contract can override that
    // to call into more app-specific code (whatever they define)
    Custom(T),
    Wasm(WasmMsg),
}

impl<T> From<CosmosMsg<T>> for cosmwasm_std::CosmosMsg<T>
where
    T: Clone + fmt::Debug + PartialEq + JsonSchema,
{
    fn from(input: CosmosMsg<T>) -> Self {
        match input {
            CosmosMsg::Bank(b) => cosmwasm_std::CosmosMsg::Bank(b),
            CosmosMsg::Custom(c) => cosmwasm_std::CosmosMsg::Custom(c),
            CosmosMsg::Wasm(w) => cosmwasm_std::CosmosMsg::Wasm(w),
        }
    }
}

impl<T> From<cosmwasm_std::CosmosMsg<T>> for CosmosMsg<T>
where
    T: Clone + fmt::Debug + PartialEq + JsonSchema,
{
    fn from(input: cosmwasm_std::CosmosMsg<T>) -> CosmosMsg<T> {
        match input {
            cosmwasm_std::CosmosMsg::Bank(b) => CosmosMsg::Bank(b),
            cosmwasm_std::CosmosMsg::Custom(c) => CosmosMsg::Custom(c),
            cosmwasm_std::CosmosMsg::Wasm(w) => CosmosMsg::Wasm(w),
            _ => panic!("Unsupported type"),
        }
    }
}

pub trait CosmosRouter {
    type ExecC;
    type QueryC: CustomQuery;

    fn execute(
        &self,
        api: &dyn Api,
        storage: &mut dyn Storage,
        block: &BlockInfo,
        sender: Addr,
        msg: CosmosMsg<Self::ExecC>,
    ) -> AnyResult<AppResponse>;

    fn query(
        &self,
        api: &dyn Api,
        storage: &dyn Storage,
        block: &BlockInfo,
        request: QueryRequest<Self::QueryC>,
    ) -> AnyResult<Binary>;
}

impl<BankT, CustomT, WasmT> CosmosRouter for Router<BankT, CustomT, WasmT>
where
    CustomT::ExecC: std::fmt::Debug + Clone + PartialEq + JsonSchema + DeserializeOwned + 'static,
    CustomT::QueryC: CustomQuery + DeserializeOwned + 'static,
    CustomT: CustomHandler,
    WasmT: Wasm<CustomT::ExecC, CustomT::QueryC>,
    BankT: Bank,
{
    type ExecC = CustomT::ExecC;
    type QueryC = CustomT::QueryC;

    fn execute(
        &self,
        api: &dyn Api,
        storage: &mut dyn Storage,
        block: &BlockInfo,
        sender: Addr,
        msg: CosmosMsg<Self::ExecC>,
    ) -> AnyResult<AppResponse> {
        match msg {
            CosmosMsg::Wasm(msg) => self.wasm.execute(api, storage, self, block, sender, msg),
            CosmosMsg::Bank(msg) => self.bank.execute(storage, sender, msg),
            CosmosMsg::Custom(msg) => self.custom.execute(api, storage, block, sender, msg),
        }
    }

    /// this is used by `RouterQuerier` to actual implement the `Querier` interface.
    /// you most likely want to use `router.querier(storage, block).wrap()` to get a
    /// QuerierWrapper to interact with
    fn query(
        &self,
        api: &dyn Api,
        storage: &dyn Storage,
        block: &BlockInfo,
        request: QueryRequest<Self::QueryC>,
    ) -> AnyResult<Binary> {
        match request {
            QueryRequest::Wasm(req) => {
                self.wasm
                    .query(api, storage, &self.querier(api, storage, block), block, req)
            }
            QueryRequest::Bank(req) => self.bank.query(api, storage, req),
            QueryRequest::Custom(req) => self.custom.query(api, storage, block, req),
            _ => unimplemented!(),
        }
    }
}

pub struct RouterQuerier<'a, ExecC, QueryC> {
    router: &'a dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
    api: &'a dyn Api,
    storage: &'a dyn Storage,
    block_info: &'a BlockInfo,
}

impl<'a, ExecC, QueryC> RouterQuerier<'a, ExecC, QueryC> {
    pub fn new(
        router: &'a dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
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

impl<'a, ExecC, QueryC> Querier for RouterQuerier<'a, ExecC, QueryC>
where
    ExecC: Clone + fmt::Debug + PartialEq + JsonSchema + DeserializeOwned + 'static,
    QueryC: CustomQuery + DeserializeOwned + 'static,
{
    fn raw_query(&self, bin_request: &[u8]) -> QuerierResult {
        let request: QueryRequest<QueryC> = match from_slice(bin_request) {
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
    use cosmwasm_std::{
        coin, coins, to_binary, AllBalanceResponse, Attribute, BankMsg, BankQuery, Event, Reply,
        StdResult, SubMsg, WasmMsg,
    };

    use crate::error::Error;
    use crate::test_helpers::contracts::{echo, hackatom, payout, reflect};
    use crate::test_helpers::{CustomMsg, EmptyMsg};
    use crate::transactions::StorageTransaction;
    use cosmwasm_std::{OverflowError, OverflowOperation, StdError};

    use super::*;

    fn get_balance<BankT, ApiT, StorageT, CustomT, WasmT>(
        app: &App<BankT, ApiT, StorageT, CustomT, WasmT>,
        addr: &Addr,
    ) -> Vec<Coin>
    where
        CustomT::ExecC: Clone + fmt::Debug + PartialEq + JsonSchema + DeserializeOwned + 'static,
        CustomT::QueryC: CustomQuery + DeserializeOwned + 'static,
        WasmT: Wasm<CustomT::ExecC, CustomT::QueryC>,
        BankT: Bank,
        ApiT: Api,
        StorageT: Storage,
        CustomT: CustomHandler,
    {
        app.wrap().query_all_balances(addr).unwrap()
    }

    #[test]
    fn update_block() {
        let mut app = App::new();

        let BlockInfo { time, height, .. } = app.block;
        app.update_block(next_block);

        assert_eq!(time.plus_seconds(5), app.block.time);
        assert_eq!(height + 1, app.block.height);
    }

    #[test]
    fn send_tokens() {
        let mut app = App::new();

        let owner = Addr::unchecked("owner");
        let rcpt = Addr::unchecked("receiver");
        let init_funds = vec![coin(20, "btc"), coin(100, "eth")];
        let rcpt_funds = vec![coin(5, "btc")];

        // set money
        app.init_bank_balance(&owner, init_funds).unwrap();
        app.init_bank_balance(&rcpt, rcpt_funds).unwrap();

        // send both tokens
        let to_send = vec![coin(30, "eth"), coin(5, "btc")];
        let msg: cosmwasm_std::CosmosMsg = BankMsg::Send {
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
        let msg: cosmwasm_std::CosmosMsg = BankMsg::Send {
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
        let mut app = App::new();

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
        assert_eq!(3, res.events.len());

        // the call to payout does emit this as well as custom attributes
        let payout_exec = &res.events[0];
        assert_eq!(payout_exec.ty.as_str(), "execute");
        assert_eq!(payout_exec.attributes, [("_contract_addr", &contract_addr)]);

        // next is a custom wasm event
        let custom_attrs = res.custom_attrs(1);
        assert_eq!(custom_attrs, [("action", "payout")]);

        // then the transfer event
        let expected_transfer = Event::new("transfer")
            .add_attribute("recipient", "random")
            .add_attribute("sender", &contract_addr)
            .add_attribute("amount", "5eth");
        assert_eq!(&expected_transfer, &res.events[2]);

        // random got cash
        let funds = get_balance(&app, &random);
        assert_eq!(funds, coins(5, "eth"));
        // contract lost it
        let funds = get_balance(&app, &contract_addr);
        assert_eq!(funds, coins(18, "eth"));
    }

    #[test]
    fn reflect_success() {
        let mut app = custom_app::<CustomMsg, Empty>();

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
        assert_eq!(4, res.events.len(), "{:?}", res.events);

        // reflect only returns standard wasm-execute event
        let ref_exec = &res.events[0];
        assert_eq!(ref_exec.ty.as_str(), "execute");
        assert_eq!(ref_exec.attributes, [("_contract_addr", &reflect_addr)]);

        // the call to payout does emit this as well as custom attributes
        let payout_exec = &res.events[1];
        assert_eq!(payout_exec.ty.as_str(), "execute");
        assert_eq!(payout_exec.attributes, [("_contract_addr", &payout_addr)]);

        let payout = &res.events[2];
        assert_eq!(payout.ty.as_str(), "wasm");
        assert_eq!(
            payout.attributes,
            [
                ("_contract_addr", payout_addr.as_str()),
                ("action", "payout")
            ]
        );

        // final event is the transfer from bank
        let second = &res.events[3];
        assert_eq!(second.ty.as_str(), "transfer");
        assert_eq!(3, second.attributes.len());
        assert_eq!(second.attributes[0], ("recipient", &reflect_addr));
        assert_eq!(second.attributes[1], ("sender", &payout_addr));
        assert_eq!(second.attributes[2], ("amount", "5eth"));

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
        let mut app = custom_app::<CustomMsg, Empty>();

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
        assert_eq!(2, res.events.len());
        // standard wasm-execute event
        let exec = &res.events[0];
        assert_eq!(exec.ty.as_str(), "execute");
        assert_eq!(exec.attributes, [("_contract_addr", &reflect_addr)]);
        // only transfer event from bank
        let transfer = &res.events[1];
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
        assert_eq!(
            StdError::overflow(OverflowError::new(OverflowOperation::Sub, 0, 3)),
            err.downcast().unwrap()
        );

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
        let mut app = App::new();

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
        let mut app = custom_app::<CustomMsg, Empty>();

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

        // expected events: execute, transfer, reply, custom wasm (set in reply)
        assert_eq!(4, res.events.len(), "{:?}", res.events);
        res.assert_event(&Event::new("execute").add_attribute("_contract_addr", &reflect_addr));
        res.assert_event(&Event::new("transfer").add_attribute("amount", "7eth"));
        res.assert_event(
            &Event::new("reply")
                .add_attribute("_contract_addr", reflect_addr.as_str())
                .add_attribute("mode", "handle_success"),
        );
        res.assert_event(&Event::new("wasm-custom").add_attribute("from", "reply"));

        // ensure success was written
        let res: Reply = app.wrap().query_wasm_smart(&reflect_addr, &query).unwrap();
        assert_eq!(res.id, 123);
        // validate the events written in the reply blob...should just be bank transfer
        let reply = res.result.unwrap();
        assert_eq!(1, reply.events.len());
        AppResponse::from(reply)
            .assert_event(&Event::new("transfer").add_attribute("amount", "7eth"));

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

    fn query_router<BankT, CustomT, WasmT>(
        router: &Router<BankT, CustomT, WasmT>,
        api: &dyn Api,
        storage: &dyn Storage,
        rcpt: &Addr,
    ) -> Vec<Coin>
    where
        CustomT::ExecC: Clone + fmt::Debug + PartialEq + JsonSchema,
        WasmT: Wasm<CustomT::ExecC, CustomT::QueryC>,
        BankT: Bank,
        CustomT: CustomHandler,
    {
        let query = BankQuery::AllBalances {
            address: rcpt.into(),
        };
        let res = router.bank.query(api, storage, query).unwrap();
        let val: AllBalanceResponse = from_slice(&res).unwrap();
        val.amount
    }

    fn query_app<BankT, ApiT, StorageT, CustomT, WasmT>(
        app: &App<BankT, ApiT, StorageT, CustomT, WasmT>,
        rcpt: &Addr,
    ) -> Vec<Coin>
    where
        CustomT::ExecC:
            std::fmt::Debug + PartialEq + Clone + JsonSchema + DeserializeOwned + 'static,
        CustomT::QueryC: CustomQuery + DeserializeOwned + 'static,
        WasmT: Wasm<CustomT::ExecC, CustomT::QueryC>,
        BankT: Bank,
        ApiT: Api,
        StorageT: Storage,
        CustomT: CustomHandler,
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
        let mut app = App::new();

        // set personal balance
        let owner = Addr::unchecked("owner");
        let rcpt = Addr::unchecked("recipient");
        let init_funds = vec![coin(20, "btc"), coin(100, "eth")];
        app.init_bank_balance(&owner, init_funds).unwrap();

        // cache 1 - send some tokens
        let mut cache = StorageTransaction::new(&app.storage);
        let msg = CosmosMsg::Bank(BankMsg::Send {
            to_address: rcpt.clone().into(),
            amount: coins(25, "eth"),
        });
        app.router
            .execute(&app.api, &mut cache, &app.block, owner.clone(), msg)
            .unwrap();

        // shows up in cache
        let cached_rcpt = query_router(&app.router, &app.api, &cache, &rcpt);
        assert_eq!(coins(25, "eth"), cached_rcpt);
        let router_rcpt = query_app(&app, &rcpt);
        assert_eq!(router_rcpt, vec![]);

        // now, second level cache
        transactional(&mut cache, |cache2, read| {
            let msg = CosmosMsg::Bank(BankMsg::Send {
                to_address: rcpt.clone().into(),
                amount: coins(12, "eth"),
            });
            app.router
                .execute(&app.api, cache2, &app.block, owner, msg)
                .unwrap();

            // shows up in 2nd cache
            let cached_rcpt = query_router(&app.router, &app.api, read, &rcpt);
            assert_eq!(coins(25, "eth"), cached_rcpt);
            let cached2_rcpt = query_router(&app.router, &app.api, cache2, &rcpt);
            assert_eq!(coins(37, "eth"), cached2_rcpt);
            Ok(())
        })
        .unwrap();

        // apply first to router
        cache.prepare().commit(&mut app.storage);

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
        let mut app = App::new();

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
        let mut app = App::new();

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

    mod reply_data_overwrite {
        use super::*;

        fn make_echo_submsg(
            contract: Addr,
            data: impl Into<Option<&'static str>>,
            sub_msg: Vec<SubMsg>,
            id: u64,
        ) -> SubMsg {
            let data = data.into().map(|s| s.to_owned());
            SubMsg::reply_always(
                CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: contract.into(),
                    msg: to_binary(&echo::Message {
                        data,
                        sub_msg,
                        ..echo::Message::default()
                    })
                    .unwrap(),
                    funds: vec![],
                }),
                id,
            )
        }

        fn make_echo_submsg_no_reply(
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
            let mut app = App::new();

            let owner = Addr::unchecked("owner");

            let contract_id = app.store_code(echo::contract());
            let contract = app
                .instantiate_contract(contract_id, owner.clone(), &EmptyMsg {}, &[], "Echo", None)
                .unwrap();

            let response = app
                .execute_contract(
                    owner,
                    contract,
                    &echo::Message::<Empty> {
                        data: Some("Data".to_owned()),
                        ..echo::Message::default()
                    },
                    &[],
                )
                .unwrap();

            assert_eq!(response.data, Some(b"Data".into()));
        }

        #[test]
        fn single_submsg() {
            let mut app = App::new();

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
                        sub_msg: vec![make_echo_submsg(contract, "Second", vec![], 1)],
                        ..echo::Message::default()
                    },
                    &[],
                )
                .unwrap();

            assert_eq!(response.data, Some(b"Second".into()));
        }

        #[test]
        fn single_submsg_no_reply() {
            let mut app = App::new();

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
                        sub_msg: vec![make_echo_submsg_no_reply(contract, "Second", vec![])],
                        ..echo::Message::default()
                    },
                    &[],
                )
                .unwrap();

            assert_eq!(response.data, Some(b"First".into()));
        }

        #[test]
        fn single_no_submsg_data() {
            let mut app = App::new();

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
                        sub_msg: vec![make_echo_submsg(contract, None, vec![], 1)],
                        ..echo::Message::default()
                    },
                    &[],
                )
                .unwrap();

            assert_eq!(response.data, Some(b"First".into()));
        }

        #[test]
        fn single_no_top_level_data() {
            let mut app = App::new();

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
                        sub_msg: vec![make_echo_submsg(contract, "Second", vec![], 1)],
                        ..echo::Message::default()
                    },
                    &[],
                )
                .unwrap();

            assert_eq!(response.data, Some(b"Second".into()));
        }

        #[test]
        fn single_submsg_reply_returns_none() {
            let mut app = custom_app::<CustomMsg, Empty>();

            // set personal balance
            let owner = Addr::unchecked("owner");
            app.init_bank_balance(&owner, coins(100, "tgd")).unwrap();

            // set up reflect contract
            let reflect_id = app.store_code(reflect::contract());
            let reflect_addr = app
                .instantiate_contract(
                    reflect_id,
                    owner.clone(),
                    &EmptyMsg {},
                    &[],
                    "Reflect",
                    None,
                )
                .unwrap();

            // set up echo contract
            let echo_id = app.store_code(echo::custom_contract());
            let echo_addr = app
                .instantiate_contract(echo_id, owner.clone(), &EmptyMsg {}, &[], "Echo", None)
                .unwrap();

            // reflect will call echo
            // echo will set the data
            // top-level app will not display the data
            let echo_msg = echo::Message::<Empty> {
                data: Some("my echo".into()),
                events: vec![Event::new("echo").add_attribute("called", "true")],
                ..echo::Message::default()
            };
            let reflect_msg = reflect::Message {
                messages: vec![SubMsg::new(WasmMsg::Execute {
                    contract_addr: echo_addr.to_string(),
                    msg: to_binary(&echo_msg).unwrap(),
                    funds: vec![],
                })],
            };

            let res = app
                .execute_contract(owner, reflect_addr.clone(), &reflect_msg, &[])
                .unwrap();

            // ensure data is empty
            assert_eq!(res.data, None);
            // ensure expected events
            assert_eq!(res.events.len(), 3, "{:?}", res.events);
            res.assert_event(&Event::new("execute").add_attribute("_contract_addr", &reflect_addr));
            res.assert_event(&Event::new("execute").add_attribute("_contract_addr", &echo_addr));
            res.assert_event(&Event::new("wasm-echo"));
        }

        #[test]
        fn multiple_submsg() {
            let mut app = App::new();

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
                            make_echo_submsg(contract.clone(), None, vec![], 1),
                            make_echo_submsg(contract.clone(), "First", vec![], 2),
                            make_echo_submsg(contract.clone(), "Second", vec![], 3),
                            make_echo_submsg(contract, None, vec![], 4),
                        ],
                        ..echo::Message::default()
                    },
                    &[],
                )
                .unwrap();

            assert_eq!(response.data, Some(b"Second".into()));
        }

        #[test]
        fn multiple_submsg_no_reply() {
            let mut app = App::new();

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
                            make_echo_submsg_no_reply(contract.clone(), None, vec![]),
                            make_echo_submsg_no_reply(contract.clone(), "First", vec![]),
                            make_echo_submsg_no_reply(contract.clone(), "Second", vec![]),
                            make_echo_submsg_no_reply(contract, None, vec![]),
                        ],
                        ..echo::Message::default()
                    },
                    &[],
                )
                .unwrap();

            assert_eq!(response.data, Some(b"Orig".into()));
        }

        #[test]
        fn multiple_submsg_mixed() {
            let mut app = App::new();

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
                        sub_msg: vec![
                            make_echo_submsg(contract.clone(), None, vec![], 1),
                            make_echo_submsg_no_reply(contract.clone(), "Hidden", vec![]),
                            make_echo_submsg(contract.clone(), "Shown", vec![], 2),
                            make_echo_submsg(contract.clone(), None, vec![], 3),
                            make_echo_submsg_no_reply(contract, "Lost", vec![]),
                        ],
                        ..echo::Message::default()
                    },
                    &[],
                )
                .unwrap();

            assert_eq!(response.data, Some(b"Shown".into()));
        }

        #[test]
        fn nested_submsg() {
            let mut app = App::new();

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
                                    vec![make_echo_submsg(contract, None, vec![], 4)],
                                    3,
                                )],
                                2,
                            )],
                            1,
                        )],
                        ..echo::Message::default()
                    },
                    &[],
                )
                .unwrap();

            assert_eq!(response.data, Some(b"Second".into()));
        }
    }

    mod response_validation {
        use super::*;

        #[test]
        fn empty_attribute_key() {
            let mut app = App::new();

            let owner = Addr::unchecked("owner");

            let contract_id = app.store_code(echo::contract());
            let contract = app
                .instantiate_contract(contract_id, owner.clone(), &EmptyMsg {}, &[], "Echo", None)
                .unwrap();

            let err = app
                .execute_contract(
                    owner,
                    contract,
                    &echo::Message::<Empty> {
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

            assert_eq!(Error::empty_attribute_key("value"), err.downcast().unwrap(),);
        }

        #[test]
        fn empty_attribute_value() {
            let mut app = App::new();

            let owner = Addr::unchecked("owner");

            let contract_id = app.store_code(echo::contract());
            let contract = app
                .instantiate_contract(contract_id, owner.clone(), &EmptyMsg {}, &[], "Echo", None)
                .unwrap();

            let err = app
                .execute_contract(
                    owner,
                    contract,
                    &echo::Message::<Empty> {
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

            assert_eq!(Error::empty_attribute_value("key"), err.downcast().unwrap());
        }

        #[test]
        fn empty_event_attribute_key() {
            let mut app = App::new();

            let owner = Addr::unchecked("owner");

            let contract_id = app.store_code(echo::contract());
            let contract = app
                .instantiate_contract(contract_id, owner.clone(), &EmptyMsg {}, &[], "Echo", None)
                .unwrap();

            let err = app
                .execute_contract(
                    owner,
                    contract,
                    &echo::Message::<Empty> {
                        data: None,
                        events: vec![Event::new("event")
                            .add_attribute("   ", "value")
                            .add_attribute("proper", "proper_val")],
                        ..echo::Message::default()
                    },
                    &[],
                )
                .unwrap_err();

            assert_eq!(Error::empty_attribute_key("value"), err.downcast().unwrap());
        }

        #[test]
        fn empty_event_attribute_value() {
            let mut app = App::new();

            let owner = Addr::unchecked("owner");

            let contract_id = app.store_code(echo::contract());
            let contract = app
                .instantiate_contract(contract_id, owner.clone(), &EmptyMsg {}, &[], "Echo", None)
                .unwrap();

            let err = app
                .execute_contract(
                    owner,
                    contract,
                    &echo::Message::<Empty> {
                        data: None,
                        events: vec![Event::new("event")
                            .add_attribute("key", "   ")
                            .add_attribute("proper", "proper_val")],
                        ..echo::Message::default()
                    },
                    &[],
                )
                .unwrap_err();

            assert_eq!(Error::empty_attribute_value("key"), err.downcast().unwrap());
        }

        #[test]
        fn too_short_event_type() {
            let mut app = App::new();

            let owner = Addr::unchecked("owner");

            let contract_id = app.store_code(echo::contract());
            let contract = app
                .instantiate_contract(contract_id, owner.clone(), &EmptyMsg {}, &[], "Echo", None)
                .unwrap();

            let err = app
                .execute_contract(
                    owner,
                    contract,
                    &echo::Message::<Empty> {
                        data: None,
                        events: vec![Event::new(" e "), Event::new("event")],
                        ..echo::Message::default()
                    },
                    &[],
                )
                .unwrap_err();

            assert_eq!(Error::event_type_too_short("e"), err.downcast().unwrap());
        }
    }

    mod custom_messages {
        use super::*;
        use crate::custom_handler::CachingCustomHandler;

        #[test]
        fn triggering_custom_msg() {
            let api = MockApi::default();
            let sender = api.addr_validate("sender").unwrap();
            let owner = api.addr_validate("owner").unwrap();

            let custom_handler = CachingCustomHandler::<CustomMsg, Empty>::new();
            let custom_handler_state = custom_handler.state();

            let mut app = AppBuilder::new_custom()
                .with_api(api)
                .with_custom(custom_handler)
                .build();

            let contract_id = app.store_code(echo::custom_contract());
            let contract = app
                .instantiate_contract(contract_id, owner, &EmptyMsg {}, &[], "Echo", None)
                .unwrap();

            app.execute_contract(
                sender,
                contract,
                &echo::Message {
                    sub_msg: vec![SubMsg::new(CosmosMsg::Custom(CustomMsg::SetAge {
                        age: 20,
                    }))],
                    ..Default::default()
                },
                &[],
            )
            .unwrap();

            assert_eq!(
                custom_handler_state.execs().to_owned(),
                vec![CustomMsg::SetAge { age: 20 }]
            );

            assert!(custom_handler_state.queries().is_empty());
        }
    }
}
