use std::fmt::{self, Debug};
use std::marker::PhantomData;

use anyhow::Result as AnyResult;
use cosmwasm_std::testing::{mock_env, MockApi, MockStorage};
use cosmwasm_std::{
    from_slice, to_binary, Addr, Api, Binary, BlockInfo, ContractResult, CustomQuery, Empty,
    Querier, QuerierResult, QuerierWrapper, QueryRequest, Storage, SystemError, SystemResult,
};
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::bank::{Bank, BankKeeper, BankSudo};
use crate::contracts::Contract;
use crate::executor::{AppResponse, Executor};
use crate::module::{FailingModule, Module};
use crate::staking::{Distribution, FailingDistribution, FailingStaking, Staking, StakingSudo};
use crate::transactions::transactional;
use crate::untyped_msg::CosmosMsg;
use crate::wasm::{ContractData, Wasm, WasmKeeper, WasmSudo};

pub fn next_block(block: &mut BlockInfo) {
    block.time = block.time.plus_seconds(5);
    block.height += 1;
}

/// Type alias for default build `App` to make its storing simpler in typical scenario
pub type BasicApp<ExecC = Empty, QueryC = Empty> = App<
    BankKeeper,
    MockApi,
    MockStorage,
    FailingModule<ExecC, QueryC, Empty>,
    WasmKeeper<ExecC, QueryC>,
>;

/// Router is a persisted state. You can query this.
/// Execution generally happens on the RouterCache, which then can be atomically committed or rolled back.
/// We offer .execute() as a wrapper around cache, execute, commit/rollback process.
pub struct App<
    Bank = BankKeeper,
    Api = MockApi,
    Storage = MockStorage,
    Custom = FailingModule<Empty, Empty, Empty>,
    Wasm = WasmKeeper<Empty, Empty>,
    Staking = FailingStaking,
    Distr = FailingDistribution,
> {
    router: Router<Bank, Custom, Wasm, Staking, Distr>,
    api: Api,
    storage: Storage,
    block: BlockInfo,
}

fn no_init<BankT, CustomT, WasmT, StakingT, DistrT>(
    _: &mut Router<BankT, CustomT, WasmT, StakingT, DistrT>,
    _: &dyn Api,
    _: &mut dyn Storage,
) {
}

impl Default for BasicApp {
    fn default() -> Self {
        Self::new(no_init)
    }
}

impl BasicApp {
    /// Creates new default `App` implementation working with Empty custom messages.
    pub fn new<F>(init_fn: F) -> Self
    where
        F: FnOnce(
            &mut Router<
                BankKeeper,
                FailingModule<Empty, Empty, Empty>,
                WasmKeeper<Empty, Empty>,
                FailingStaking,
                FailingDistribution,
            >,
            &dyn Api,
            &mut dyn Storage,
        ),
    {
        AppBuilder::new().build(init_fn)
    }
}

/// Creates new default `App` implementation working with customized exec and query messages.
/// Outside of `App` implementation to make type elision better.
pub fn custom_app<ExecC, QueryC, F>(init_fn: F) -> BasicApp<ExecC, QueryC>
where
    ExecC: Debug + Clone + PartialEq + JsonSchema + DeserializeOwned + 'static,
    QueryC: Debug + CustomQuery + DeserializeOwned + 'static,
    F: FnOnce(
        &mut Router<
            BankKeeper,
            FailingModule<ExecC, QueryC, Empty>,
            WasmKeeper<ExecC, QueryC>,
            FailingStaking,
            FailingDistribution,
        >,
        &dyn Api,
        &mut dyn Storage,
    ),
{
    AppBuilder::new_custom().build(init_fn)
}

impl<BankT, ApiT, StorageT, CustomT, WasmT, StakingT, DistrT> Querier
    for App<BankT, ApiT, StorageT, CustomT, WasmT, StakingT, DistrT>
where
    CustomT::ExecT: Clone + fmt::Debug + PartialEq + JsonSchema + DeserializeOwned + 'static,
    CustomT::QueryT: CustomQuery + DeserializeOwned + 'static,
    WasmT: Wasm<CustomT::ExecT, CustomT::QueryT>,
    BankT: Bank,
    ApiT: Api,
    StorageT: Storage,
    CustomT: Module,
    StakingT: Staking,
    DistrT: Distribution,
{
    fn raw_query(&self, bin_request: &[u8]) -> QuerierResult {
        self.router
            .querier(&self.api, &self.storage, &self.block)
            .raw_query(bin_request)
    }
}

impl<BankT, ApiT, StorageT, CustomT, WasmT, StakingT, DistrT> Executor<CustomT::ExecT>
    for App<BankT, ApiT, StorageT, CustomT, WasmT, StakingT, DistrT>
where
    CustomT::ExecT: Clone + fmt::Debug + PartialEq + JsonSchema + DeserializeOwned + 'static,
    CustomT::QueryT: CustomQuery + DeserializeOwned + 'static,
    WasmT: Wasm<CustomT::ExecT, CustomT::QueryT>,
    BankT: Bank,
    ApiT: Api,
    StorageT: Storage,
    CustomT: Module,
    StakingT: Staking,
    DistrT: Distribution,
{
    fn execute(
        &mut self,
        sender: Addr,
        msg: cosmwasm_std::CosmosMsg<CustomT::ExecT>,
    ) -> AnyResult<AppResponse> {
        let mut all = self.execute_multi(sender, vec![msg])?;
        let res = all.pop().unwrap();
        Ok(res)
    }
}

/// This is essential to create a custom app with custom handler.
///   let mut app = BasicAppBuilder::<E, Q>::new_custom().with_custom(handler).build();
pub type BasicAppBuilder<ExecC, QueryC> = AppBuilder<
    BankKeeper,
    MockApi,
    MockStorage,
    FailingModule<ExecC, QueryC, Empty>,
    WasmKeeper<ExecC, QueryC>,
    FailingStaking,
    FailingDistribution,
>;

/// Utility to build App in stages. If particular items wont be set, defaults would be used
pub struct AppBuilder<Bank, Api, Storage, Custom, Wasm, Staking, Distr> {
    api: Api,
    block: BlockInfo,
    storage: Storage,
    bank: Bank,
    wasm: Wasm,
    custom: Custom,
    staking: Staking,
    distribution: Distr,
}

impl Default
    for AppBuilder<
        BankKeeper,
        MockApi,
        MockStorage,
        FailingModule<Empty, Empty, Empty>,
        WasmKeeper<Empty, Empty>,
        FailingStaking,
        FailingDistribution,
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
        FailingModule<Empty, Empty, Empty>,
        WasmKeeper<Empty, Empty>,
        FailingStaking,
        FailingDistribution,
    >
{
    /// Creates builder with default components working with empty exec and query messages.
    pub fn new() -> Self {
        AppBuilder {
            api: MockApi::default(),
            block: mock_env().block,
            storage: MockStorage::new(),
            bank: BankKeeper::new(),
            wasm: WasmKeeper::new(),
            custom: FailingModule::new(),
            staking: FailingStaking::new(),
            distribution: FailingDistribution::new(),
        }
    }
}

impl<ExecC, QueryC>
    AppBuilder<
        BankKeeper,
        MockApi,
        MockStorage,
        FailingModule<ExecC, QueryC, Empty>,
        WasmKeeper<ExecC, QueryC>,
        FailingStaking,
        FailingDistribution,
    >
where
    ExecC: Debug + Clone + PartialEq + JsonSchema + DeserializeOwned + 'static,
    QueryC: Debug + CustomQuery + DeserializeOwned + 'static,
{
    /// Creates builder with default components designed to work with custom exec and query
    /// messages.
    pub fn new_custom() -> Self {
        AppBuilder {
            api: MockApi::default(),
            block: mock_env().block,
            storage: MockStorage::new(),
            bank: BankKeeper::new(),
            wasm: WasmKeeper::new(),
            custom: FailingModule::new(),
            staking: FailingStaking::new(),
            distribution: FailingDistribution::new(),
        }
    }
}

impl<BankT, ApiT, StorageT, CustomT, WasmT, StakingT, DistrT>
    AppBuilder<BankT, ApiT, StorageT, CustomT, WasmT, StakingT, DistrT>
{
    /// Overwrites default wasm executor.
    ///
    /// At this point it is needed that new wasm implements some `Wasm` trait, but it doesn't need
    /// to be bound to Bank or Custom yet - as those may change. The cross-components validation is
    /// done on final building.
    ///
    /// Also it is possible to completely abandon trait bounding here which would not be bad idea,
    /// however it might make the message on build creepy in many cases, so as for properly build
    /// `App` we always want `Wasm` to be `Wasm`, some checks are done early.
    pub fn with_wasm<C: Module, NewWasm: Wasm<C::ExecT, C::QueryT>>(
        self,
        wasm: NewWasm,
    ) -> AppBuilder<BankT, ApiT, StorageT, CustomT, NewWasm, StakingT, DistrT> {
        let AppBuilder {
            bank,
            api,
            storage,
            custom,
            block,
            staking,
            distribution,
            ..
        } = self;

        AppBuilder {
            api,
            block,
            storage,
            bank,
            wasm,
            custom,
            staking,
            distribution,
        }
    }

    /// Overwrites default bank interface
    pub fn with_bank<NewBank: Bank>(
        self,
        bank: NewBank,
    ) -> AppBuilder<NewBank, ApiT, StorageT, CustomT, WasmT, StakingT, DistrT> {
        let AppBuilder {
            wasm,
            api,
            storage,
            custom,
            block,
            staking,
            distribution,
            ..
        } = self;

        AppBuilder {
            api,
            block,
            storage,
            bank,
            wasm,
            custom,
            staking,
            distribution,
        }
    }

    /// Overwrites default api interface
    pub fn with_api<NewApi: Api>(
        self,
        api: NewApi,
    ) -> AppBuilder<BankT, NewApi, StorageT, CustomT, WasmT, StakingT, DistrT> {
        let AppBuilder {
            wasm,
            bank,
            storage,
            custom,
            block,
            staking,
            distribution,
            ..
        } = self;

        AppBuilder {
            api,
            block,
            storage,
            bank,
            wasm,
            custom,
            staking,
            distribution,
        }
    }

    /// Overwrites default storage interface
    pub fn with_storage<NewStorage: Storage>(
        self,
        storage: NewStorage,
    ) -> AppBuilder<BankT, ApiT, NewStorage, CustomT, WasmT, StakingT, DistrT> {
        let AppBuilder {
            wasm,
            api,
            bank,
            custom,
            block,
            staking,
            distribution,
            ..
        } = self;

        AppBuilder {
            api,
            block,
            storage,
            bank,
            wasm,
            custom,
            staking,
            distribution,
        }
    }

    /// Overwrites default custom messages handler
    ///
    /// At this point it is needed that new custom implements some `Module` trait, but it doesn't need
    /// to be bound to ExecC or QueryC yet - as those may change. The cross-components validation is
    /// done on final building.
    ///
    /// Also it is possible to completely abandon trait bounding here which would not be bad idea,
    /// however it might make the message on build creepy in many cases, so as for properly build
    /// `App` we always want `Wasm` to be `Wasm`, some checks are done early.
    pub fn with_custom<NewCustom: Module>(
        self,
        custom: NewCustom,
    ) -> AppBuilder<BankT, ApiT, StorageT, NewCustom, WasmT, StakingT, DistrT> {
        let AppBuilder {
            wasm,
            bank,
            api,
            storage,
            block,
            staking,
            distribution,
            ..
        } = self;

        AppBuilder {
            api,
            block,
            storage,
            bank,
            wasm,
            custom,
            staking,
            distribution,
        }
    }

    /// Overwrites default bank interface
    pub fn with_staking<NewStaking: Staking>(
        self,
        staking: NewStaking,
    ) -> AppBuilder<BankT, ApiT, StorageT, CustomT, WasmT, NewStaking, DistrT> {
        let AppBuilder {
            wasm,
            api,
            storage,
            custom,
            block,
            bank,
            distribution,
            ..
        } = self;

        AppBuilder {
            api,
            block,
            storage,
            bank,
            wasm,
            custom,
            staking,
            distribution,
        }
    }

    /// Overwrites default bank interface
    pub fn with_distribution<NewDistribution: Distribution>(
        self,
        distribution: NewDistribution,
    ) -> AppBuilder<BankT, ApiT, StorageT, CustomT, WasmT, StakingT, NewDistribution> {
        let AppBuilder {
            wasm,
            api,
            storage,
            custom,
            block,
            staking,
            bank,
            ..
        } = self;

        AppBuilder {
            api,
            block,
            storage,
            bank,
            wasm,
            custom,
            staking,
            distribution,
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
    pub fn build<F>(
        self,
        init_fn: F,
    ) -> App<BankT, ApiT, StorageT, CustomT, WasmT, StakingT, DistrT>
    where
        BankT: Bank,
        ApiT: Api,
        StorageT: Storage,
        CustomT: Module,
        WasmT: Wasm<CustomT::ExecT, CustomT::QueryT>,
        StakingT: Staking,
        DistrT: Distribution,
        F: FnOnce(&mut Router<BankT, CustomT, WasmT, StakingT, DistrT>, &dyn Api, &mut dyn Storage),
    {
        let router = Router {
            wasm: self.wasm,
            bank: self.bank,
            custom: self.custom,
            staking: self.staking,
            distribution: self.distribution,
        };

        let mut app = App {
            router,
            api: self.api,
            block: self.block,
            storage: self.storage,
        };
        app.init_modules(init_fn);
        app
    }
}

impl<BankT, ApiT, StorageT, CustomT, WasmT, StakingT, DistrT>
    App<BankT, ApiT, StorageT, CustomT, WasmT, StakingT, DistrT>
where
    WasmT: Wasm<CustomT::ExecT, CustomT::QueryT>,
    BankT: Bank,
    ApiT: Api,
    StorageT: Storage,
    CustomT: Module,
    StakingT: Staking,
    DistrT: Distribution,
{
    pub fn init_modules<F, T>(&mut self, init_fn: F) -> T
    where
        F: FnOnce(
            &mut Router<BankT, CustomT, WasmT, StakingT, DistrT>,
            &dyn Api,
            &mut dyn Storage,
        ) -> T,
    {
        init_fn(&mut self.router, &self.api, &mut self.storage)
    }

    pub fn read_module<F, T>(&self, query_fn: F) -> T
    where
        F: FnOnce(&Router<BankT, CustomT, WasmT, StakingT, DistrT>, &dyn Api, &dyn Storage) -> T,
    {
        query_fn(&self.router, &self.api, &self.storage)
    }
}

// Helper functions to call some custom WasmKeeper logic.
// They show how we can easily add such calls to other custom keepers (CustomT, StakingT, etc)
impl<BankT, ApiT, StorageT, CustomT, StakingT, DistrT>
    App<
        BankT,
        ApiT,
        StorageT,
        CustomT,
        WasmKeeper<CustomT::ExecT, CustomT::QueryT>,
        StakingT,
        DistrT,
    >
where
    BankT: Bank,
    ApiT: Api,
    StorageT: Storage,
    CustomT: Module,
    StakingT: Staking,
    DistrT: Distribution,
    CustomT::ExecT: Clone + fmt::Debug + PartialEq + JsonSchema + DeserializeOwned + 'static,
    CustomT::QueryT: CustomQuery + DeserializeOwned + 'static,
{
    /// This registers contract code (like uploading wasm bytecode on a chain),
    /// so it can later be used to instantiate a contract.
    pub fn store_code(&mut self, code: Box<dyn Contract<CustomT::ExecT, CustomT::QueryT>>) -> u64 {
        self.init_modules(|router, _, _| router.wasm.store_code(code) as u64)
    }

    /// This allows to get `ContractData` for specific contract
    pub fn contract_data(&self, address: &Addr) -> AnyResult<ContractData> {
        self.read_module(|router, _, storage| router.wasm.load_contract(storage, address))
    }
}

impl<BankT, ApiT, StorageT, CustomT, WasmT, StakingT, DistrT>
    App<BankT, ApiT, StorageT, CustomT, WasmT, StakingT, DistrT>
where
    CustomT::ExecT: std::fmt::Debug + PartialEq + Clone + JsonSchema + DeserializeOwned + 'static,
    CustomT::QueryT: CustomQuery + DeserializeOwned + 'static,
    WasmT: Wasm<CustomT::ExecT, CustomT::QueryT>,
    BankT: Bank,
    ApiT: Api,
    StorageT: Storage,
    CustomT: Module,
    StakingT: Staking,
    DistrT: Distribution,
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
    pub fn wrap(&self) -> QuerierWrapper<CustomT::QueryT> {
        QuerierWrapper::new(self)
    }

    /// Runs multiple CosmosMsg in one atomic operation.
    /// This will create a cache before the execution, so no state changes are persisted if any of them
    /// return an error. But all writes are persisted on success.
    pub fn execute_multi(
        &mut self,
        sender: Addr,
        msgs: Vec<cosmwasm_std::CosmosMsg<CustomT::ExecT>>,
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

    /// Call a smart contract in "sudo" mode.
    /// This will create a cache before the execution, so no state changes are persisted if this
    /// returns an error, but all are persisted on success.
    pub fn wasm_sudo<T: Serialize, U: Into<Addr>>(
        &mut self,
        contract_addr: U,
        msg: &T,
    ) -> AnyResult<AppResponse> {
        let msg = to_binary(msg)?;

        let Self {
            block,
            router,
            api,
            storage,
        } = self;

        transactional(&mut *storage, |write_cache, _| {
            router
                .wasm
                .sudo(&*api, contract_addr.into(), write_cache, router, block, msg)
        })
    }

    /// Runs arbitrary SudoMsg.
    /// This will create a cache before the execution, so no state changes are persisted if this
    /// returns an error, but all are persisted on success.
    pub fn sudo(&mut self, msg: SudoMsg) -> AnyResult<AppResponse> {
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
            router.sudo(&*api, write_cache, block, msg)
        })
    }
}

pub struct Router<Bank, Custom, Wasm, Staking, Distr> {
    // this can remain crate-only as all special functions are wired up to app currently
    // we need to figure out another format for wasm, as some like sudo need to be called after init
    pub(crate) wasm: Wasm,
    // these must be pub so we can initialize them (super user) on build
    pub bank: Bank,
    pub custom: Custom,
    pub staking: Staking,
    pub distribution: Distr,
}

impl<BankT, CustomT, WasmT, StakingT, DistrT> Router<BankT, CustomT, WasmT, StakingT, DistrT>
where
    CustomT::ExecT: Clone + fmt::Debug + PartialEq + JsonSchema + DeserializeOwned + 'static,
    CustomT::QueryT: CustomQuery + DeserializeOwned + 'static,
    CustomT: Module,
    WasmT: Wasm<CustomT::ExecT, CustomT::QueryT>,
    BankT: Bank,
    StakingT: Staking,
    DistrT: Distribution,
{
    pub fn querier<'a>(
        &'a self,
        api: &'a dyn Api,
        storage: &'a dyn Storage,
        block_info: &'a BlockInfo,
    ) -> RouterQuerier<'a, CustomT::ExecT, CustomT::QueryT> {
        RouterQuerier {
            router: self,
            api,
            storage,
            block_info,
        }
    }
}

/// We use it to allow calling into modules from another module in sudo mode.
/// Things like gov proposals belong here.
pub enum SudoMsg {
    Bank(BankSudo),
    Custom(Empty),
    Staking(StakingSudo),
    Wasm(WasmSudo),
}

impl From<WasmSudo> for SudoMsg {
    fn from(wasm: WasmSudo) -> Self {
        SudoMsg::Wasm(wasm)
    }
}

impl From<BankSudo> for SudoMsg {
    fn from(bank: BankSudo) -> Self {
        SudoMsg::Bank(bank)
    }
}

impl From<StakingSudo> for SudoMsg {
    fn from(staking: StakingSudo) -> Self {
        SudoMsg::Staking(staking)
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

    fn sudo(
        &self,
        api: &dyn Api,
        storage: &mut dyn Storage,
        block: &BlockInfo,
        msg: SudoMsg,
    ) -> AnyResult<AppResponse>;
}

impl<BankT, CustomT, WasmT, StakingT, DistrT> CosmosRouter
    for Router<BankT, CustomT, WasmT, StakingT, DistrT>
where
    CustomT::ExecT: std::fmt::Debug + Clone + PartialEq + JsonSchema + DeserializeOwned + 'static,
    CustomT::QueryT: CustomQuery + DeserializeOwned + 'static,
    CustomT: Module,
    WasmT: Wasm<CustomT::ExecT, CustomT::QueryT>,
    BankT: Bank,
    StakingT: Staking,
    DistrT: Distribution,
{
    type ExecC = CustomT::ExecT;
    type QueryC = CustomT::QueryT;

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
            CosmosMsg::Bank(msg) => self.bank.execute(api, storage, self, block, sender, msg),
            CosmosMsg::Custom(msg) => self.custom.execute(api, storage, self, block, sender, msg),
            CosmosMsg::Staking(msg) => self.staking.execute(api, storage, self, block, sender, msg),
            CosmosMsg::Distribution(msg) => self
                .distribution
                .execute(api, storage, self, block, sender, msg),
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
        let querier = self.querier(api, storage, block);
        match request {
            QueryRequest::Wasm(req) => self.wasm.query(api, storage, &querier, block, req),
            QueryRequest::Bank(req) => self.bank.query(api, storage, &querier, block, req),
            QueryRequest::Custom(req) => self.custom.query(api, storage, &querier, block, req),
            QueryRequest::Staking(req) => self.staking.query(api, storage, &querier, block, req),
            _ => unimplemented!(),
        }
    }

    fn sudo(
        &self,
        api: &dyn Api,
        storage: &mut dyn Storage,
        block: &BlockInfo,
        msg: SudoMsg,
    ) -> AnyResult<AppResponse> {
        match msg {
            SudoMsg::Wasm(msg) => {
                self.wasm
                    .sudo(api, msg.contract_addr, storage, self, block, msg.msg)
            }
            SudoMsg::Bank(msg) => self.bank.sudo(api, storage, self, block, msg),
            SudoMsg::Staking(msg) => self.staking.sudo(api, storage, self, block, msg),
            SudoMsg::Custom(_) => unimplemented!(),
        }
    }
}

pub struct MockRouter<ExecC, QueryC>(PhantomData<(ExecC, QueryC)>);

impl Default for MockRouter<Empty, Empty> {
    fn default() -> Self {
        Self::new()
    }
}

impl<ExecC, QueryC> MockRouter<ExecC, QueryC> {
    pub fn new() -> Self
    where
        QueryC: CustomQuery,
    {
        MockRouter(PhantomData)
    }
}

impl<ExecC, QueryC> CosmosRouter for MockRouter<ExecC, QueryC>
where
    QueryC: CustomQuery,
{
    type ExecC = ExecC;
    type QueryC = QueryC;

    fn execute(
        &self,
        _api: &dyn Api,
        _storage: &mut dyn Storage,
        _block: &BlockInfo,
        _sender: Addr,
        _msg: CosmosMsg<Self::ExecC>,
    ) -> AnyResult<AppResponse> {
        panic!("Cannot execute MockRouters");
    }

    fn query(
        &self,
        _api: &dyn Api,
        _storage: &dyn Storage,
        _block: &BlockInfo,
        _request: QueryRequest<Self::QueryC>,
    ) -> AnyResult<Binary> {
        panic!("Cannot query MockRouters");
    }

    fn sudo(
        &self,
        _api: &dyn Api,
        _storage: &mut dyn Storage,
        _block: &BlockInfo,
        _msg: SudoMsg,
    ) -> AnyResult<AppResponse> {
        panic!("Cannot sudo MockRouters");
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
                    error: format!("Parsing query request: {}", e),
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
    use super::*;
    use cosmwasm_std::testing::MockQuerier;
    use cosmwasm_std::{
        coin, coins, to_binary, AllBalanceResponse, Attribute, BankMsg, BankQuery, Coin, Event,
        OverflowError, OverflowOperation, Reply, StdError, StdResult, SubMsg, WasmMsg,
    };

    use crate::error::Error;
    use crate::test_helpers::contracts::{caller, echo, error, hackatom, payout, reflect};
    use crate::test_helpers::{CustomMsg, EmptyMsg};
    use crate::transactions::StorageTransaction;

    fn get_balance<BankT, ApiT, StorageT, CustomT, WasmT>(
        app: &App<BankT, ApiT, StorageT, CustomT, WasmT>,
        addr: &Addr,
    ) -> Vec<Coin>
    where
        CustomT::ExecT: Clone + fmt::Debug + PartialEq + JsonSchema + DeserializeOwned + 'static,
        CustomT::QueryT: CustomQuery + DeserializeOwned + 'static,
        WasmT: Wasm<CustomT::ExecT, CustomT::QueryT>,
        BankT: Bank,
        ApiT: Api,
        StorageT: Storage,
        CustomT: Module,
    {
        app.wrap().query_all_balances(addr).unwrap()
    }

    #[test]
    fn update_block() {
        let mut app = App::default();

        let BlockInfo { time, height, .. } = app.block;
        app.update_block(next_block);

        assert_eq!(time.plus_seconds(5), app.block.time);
        assert_eq!(height + 1, app.block.height);
    }

    #[test]
    fn send_tokens() {
        let owner = Addr::unchecked("owner");
        let rcpt = Addr::unchecked("receiver");
        let init_funds = vec![coin(20, "btc"), coin(100, "eth")];
        let rcpt_funds = vec![coin(5, "btc")];

        let mut app = App::new(|router, _, storage| {
            // initialization moved to App construction
            router
                .bank
                .init_balance(storage, &owner, init_funds)
                .unwrap();
            router
                .bank
                .init_balance(storage, &rcpt, rcpt_funds)
                .unwrap();
        });

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
        // set personal balance
        let owner = Addr::unchecked("owner");
        let init_funds = vec![coin(20, "btc"), coin(100, "eth")];

        let mut app = App::new(|router, _, storage| {
            router
                .bank
                .init_balance(storage, &owner, init_funds)
                .unwrap();
        });

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
        // set personal balance
        let owner = Addr::unchecked("owner");
        let init_funds = vec![coin(20, "btc"), coin(100, "eth")];

        let mut app = custom_app::<CustomMsg, Empty, _>(|router, _, storage| {
            router
                .bank
                .init_balance(storage, &owner, init_funds)
                .unwrap();
        });

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
        // set personal balance
        let owner = Addr::unchecked("owner");
        let init_funds = vec![coin(20, "btc"), coin(100, "eth")];

        let mut app = custom_app::<CustomMsg, Empty, _>(|router, _, storage| {
            router
                .bank
                .init_balance(storage, &owner, init_funds)
                .unwrap();
        });

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
        let owner = Addr::unchecked("owner");
        let init_funds = vec![coin(100, "eth")];

        let mut app = App::new(|router, _, storage| {
            router
                .bank
                .init_balance(storage, &owner, init_funds)
                .unwrap();
        });

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

        // wasm_sudo call
        let msg = payout::SudoMsg { set_count: 25 };
        app.wasm_sudo(payout_addr.clone(), &msg).unwrap();

        // count is 25
        let payout::CountResponse { count } = app
            .wrap()
            .query_wasm_smart(&payout_addr, &payout::QueryMsg::Count {})
            .unwrap();
        assert_eq!(25, count);

        // we can do the same with sudo call
        let msg = payout::SudoMsg { set_count: 49 };
        let sudo_msg = WasmSudo {
            contract_addr: payout_addr.clone(),
            msg: to_binary(&msg).unwrap(),
        };
        app.sudo(sudo_msg.into()).unwrap();

        let payout::CountResponse { count } = app
            .wrap()
            .query_wasm_smart(&payout_addr, &payout::QueryMsg::Count {})
            .unwrap();
        assert_eq!(49, count);
    }

    // this demonstrates that we can mint tokens and send from other accounts via a custom module,
    // as an example of ability to do privileged actions
    mod custom_handler {
        use super::*;

        use anyhow::{bail, Result as AnyResult};
        use cw_storage_plus::Item;
        use serde::{Deserialize, Serialize};

        use crate::Executor;

        const LOTTERY: Item<Coin> = Item::new("lottery");
        const PITY: Item<Coin> = Item::new("pity");

        #[derive(Clone, std::fmt::Debug, PartialEq, JsonSchema, Serialize, Deserialize)]
        struct CustomMsg {
            // we mint LOTTERY tokens to this one
            lucky_winner: String,
            // we transfer PITY from lucky_winner to runner_up
            runner_up: String,
        }

        struct CustomHandler {}

        impl Module for CustomHandler {
            type ExecT = CustomMsg;
            type QueryT = Empty;
            type SudoT = Empty;

            fn execute<ExecC, QueryC>(
                &self,
                api: &dyn Api,
                storage: &mut dyn Storage,
                router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
                block: &BlockInfo,
                _sender: Addr,
                msg: Self::ExecT,
            ) -> AnyResult<AppResponse>
            where
                ExecC:
                    std::fmt::Debug + Clone + PartialEq + JsonSchema + DeserializeOwned + 'static,
                QueryC: CustomQuery + DeserializeOwned + 'static,
            {
                let lottery = LOTTERY.load(storage)?;
                let pity = PITY.load(storage)?;

                // mint new tokens
                let mint = BankSudo::Mint {
                    to_address: msg.lucky_winner.clone(),
                    amount: vec![lottery],
                };
                router.sudo(api, storage, block, mint.into())?;

                // send from an arbitrary account (not the module)
                let transfer = BankMsg::Send {
                    to_address: msg.runner_up,
                    amount: vec![pity],
                };
                let rcpt = api.addr_validate(&msg.lucky_winner)?;
                router.execute(api, storage, block, rcpt, transfer.into())?;

                Ok(AppResponse::default())
            }

            fn sudo<ExecC, QueryC>(
                &self,
                _api: &dyn Api,
                _storage: &mut dyn Storage,
                _router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
                _block: &BlockInfo,
                _msg: Self::SudoT,
            ) -> AnyResult<AppResponse>
            where
                ExecC:
                    std::fmt::Debug + Clone + PartialEq + JsonSchema + DeserializeOwned + 'static,
                QueryC: CustomQuery + DeserializeOwned + 'static,
            {
                bail!("sudo not implemented for CustomHandler")
            }

            fn query(
                &self,
                _api: &dyn Api,
                _storage: &dyn Storage,
                _querier: &dyn Querier,
                _block: &BlockInfo,
                _request: Self::QueryT,
            ) -> AnyResult<Binary> {
                bail!("query not implemented for CustomHandler")
            }
        }

        impl CustomHandler {
            // this is a custom initialization method
            pub fn set_payout(
                &self,
                storage: &mut dyn Storage,
                lottery: Coin,
                pity: Coin,
            ) -> AnyResult<()> {
                LOTTERY.save(storage, &lottery)?;
                PITY.save(storage, &pity)?;
                Ok(())
            }
        }

        // let's call this custom handler
        #[test]
        fn dispatches_messages() {
            let winner = "winner".to_string();
            let second = "second".to_string();

            // payments. note 54321 - 12321 = 42000
            let denom = "tix";
            let lottery = coin(54321, denom);
            let bonus = coin(12321, denom);

            let mut app = BasicAppBuilder::<CustomMsg, Empty>::new_custom()
                .with_custom(CustomHandler {})
                .build(|router, _, storage| {
                    router
                        .custom
                        .set_payout(storage, lottery.clone(), bonus.clone())
                        .unwrap();
                });

            // query that balances are empty
            let start = app.wrap().query_balance(&winner, denom).unwrap();
            assert_eq!(start, coin(0, denom));

            // trigger the custom module
            let msg = CosmosMsg::Custom(CustomMsg {
                lucky_winner: winner.clone(),
                runner_up: second.clone(),
            });
            app.execute(Addr::unchecked("anyone"), msg.into()).unwrap();

            // see if coins were properly added
            let big_win = app.wrap().query_balance(&winner, denom).unwrap();
            assert_eq!(big_win, coin(42000, denom));
            let little_win = app.wrap().query_balance(&second, denom).unwrap();
            assert_eq!(little_win, bonus);
        }
    }

    #[test]
    fn reflect_submessage_reply_works() {
        // set personal balance
        let owner = Addr::unchecked("owner");
        let random = Addr::unchecked("random");
        let init_funds = vec![coin(20, "btc"), coin(100, "eth")];

        let mut app = custom_app::<CustomMsg, Empty, _>(|router, _, storage| {
            router
                .bank
                .init_balance(storage, &owner, init_funds)
                .unwrap();
        });

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

    fn query_router<BankT, CustomT, WasmT, StakingT, DistrT>(
        router: &Router<BankT, CustomT, WasmT, StakingT, DistrT>,
        api: &dyn Api,
        storage: &dyn Storage,
        rcpt: &Addr,
    ) -> Vec<Coin>
    where
        CustomT::ExecT: Clone + fmt::Debug + PartialEq + JsonSchema,
        CustomT::QueryT: CustomQuery + DeserializeOwned,
        WasmT: Wasm<CustomT::ExecT, CustomT::QueryT>,
        BankT: Bank,
        CustomT: Module,
        StakingT: Staking,
        DistrT: Distribution,
    {
        let query = BankQuery::AllBalances {
            address: rcpt.into(),
        };
        let block = mock_env().block;
        let querier: MockQuerier<CustomT::QueryT> = MockQuerier::new(&[]);
        let res = router
            .bank
            .query(api, storage, &querier, &block, query)
            .unwrap();
        let val: AllBalanceResponse = from_slice(&res).unwrap();
        val.amount
    }

    fn query_app<BankT, ApiT, StorageT, CustomT, WasmT, StakingT, DistrT>(
        app: &App<BankT, ApiT, StorageT, CustomT, WasmT, StakingT, DistrT>,
        rcpt: &Addr,
    ) -> Vec<Coin>
    where
        CustomT::ExecT:
            std::fmt::Debug + PartialEq + Clone + JsonSchema + DeserializeOwned + 'static,
        CustomT::QueryT: CustomQuery + DeserializeOwned + 'static,
        WasmT: Wasm<CustomT::ExecT, CustomT::QueryT>,
        BankT: Bank,
        ApiT: Api,
        StorageT: Storage,
        CustomT: Module,
        StakingT: Staking,
        DistrT: Distribution,
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
        // set personal balance
        let owner = Addr::unchecked("owner");
        let rcpt = Addr::unchecked("recipient");
        let init_funds = vec![coin(20, "btc"), coin(100, "eth")];

        let mut app = App::new(|router, _, storage| {
            router
                .bank
                .init_balance(storage, &owner, init_funds)
                .unwrap();
        });

        // cache 1 - send some tokens
        let mut cache = StorageTransaction::new(&app.storage);
        let msg = BankMsg::Send {
            to_address: rcpt.clone().into(),
            amount: coins(25, "eth"),
        };
        app.router
            .execute(&app.api, &mut cache, &app.block, owner.clone(), msg.into())
            .unwrap();

        // shows up in cache
        let cached_rcpt = query_router(&app.router, &app.api, &cache, &rcpt);
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
                .execute(&app.api, cache2, &app.block, owner, msg.into())
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
        let owner = Addr::unchecked("owner");
        let beneficiary = Addr::unchecked("beneficiary");
        let init_funds = coins(30, "btc");

        let mut app = App::new(|router, _, storage| {
            router
                .bank
                .init_balance(storage, &owner, init_funds)
                .unwrap();
        });

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
        let owner = Addr::unchecked("owner");
        let beneficiary = Addr::unchecked("beneficiary");
        let init_funds = coins(30, "btc");

        let mut app = App::new(|router, _, storage| {
            router
                .bank
                .init_balance(storage, &owner, init_funds)
                .unwrap();
        });

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

        use echo::EXECUTE_REPLY_BASE_ID;

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
            let mut app = App::default();

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
            let mut app = App::default();

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
                        sub_msg: vec![make_echo_submsg(
                            contract,
                            "Second",
                            vec![],
                            EXECUTE_REPLY_BASE_ID,
                        )],
                        ..echo::Message::default()
                    },
                    &[],
                )
                .unwrap();

            assert_eq!(response.data, Some(b"Second".into()));
        }

        #[test]
        fn single_submsg_no_reply() {
            let mut app = App::default();

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
            let mut app = App::default();

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
            let mut app = App::default();

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
                        sub_msg: vec![make_echo_submsg(
                            contract,
                            "Second",
                            vec![],
                            EXECUTE_REPLY_BASE_ID,
                        )],
                        ..echo::Message::default()
                    },
                    &[],
                )
                .unwrap();

            assert_eq!(response.data, Some(b"Second".into()));
        }

        #[test]
        fn single_submsg_reply_returns_none() {
            // set personal balance
            let owner = Addr::unchecked("owner");
            let init_funds = coins(100, "tgd");

            let mut app = custom_app::<CustomMsg, Empty, _>(|router, _, storage| {
                router
                    .bank
                    .init_balance(storage, &owner, init_funds)
                    .unwrap();
            });

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
            let mut app = App::default();

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
                            make_echo_submsg(
                                contract.clone(),
                                None,
                                vec![],
                                EXECUTE_REPLY_BASE_ID + 1,
                            ),
                            make_echo_submsg(
                                contract.clone(),
                                "First",
                                vec![],
                                EXECUTE_REPLY_BASE_ID + 2,
                            ),
                            make_echo_submsg(
                                contract.clone(),
                                "Second",
                                vec![],
                                EXECUTE_REPLY_BASE_ID + 3,
                            ),
                            make_echo_submsg(contract, None, vec![], EXECUTE_REPLY_BASE_ID + 4),
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
            let mut app = App::default();

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
            let mut app = App::default();

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
                            make_echo_submsg(
                                contract.clone(),
                                None,
                                vec![],
                                EXECUTE_REPLY_BASE_ID + 1,
                            ),
                            make_echo_submsg_no_reply(contract.clone(), "Hidden", vec![]),
                            make_echo_submsg(
                                contract.clone(),
                                "Shown",
                                vec![],
                                EXECUTE_REPLY_BASE_ID + 2,
                            ),
                            make_echo_submsg(
                                contract.clone(),
                                None,
                                vec![],
                                EXECUTE_REPLY_BASE_ID + 3,
                            ),
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
            let mut app = App::default();

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
                                    vec![make_echo_submsg(
                                        contract,
                                        None,
                                        vec![],
                                        EXECUTE_REPLY_BASE_ID + 4,
                                    )],
                                    EXECUTE_REPLY_BASE_ID + 3,
                                )],
                                EXECUTE_REPLY_BASE_ID + 2,
                            )],
                            EXECUTE_REPLY_BASE_ID + 1,
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
            let mut app = App::default();

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
            let mut app = App::default();

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
            let mut app = App::default();

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
            let mut app = App::default();

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
            let mut app = App::default();

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
                .build(no_init);

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

    mod protobuf_wrapped_data {
        use super::*;
        use crate::test_helpers::contracts::echo::EXECUTE_REPLY_BASE_ID;
        use cw_utils::parse_instantiate_response_data;

        #[test]
        fn instantiate_wrapped_properly() {
            // set personal balance
            let owner = Addr::unchecked("owner");
            let init_funds = vec![coin(20, "btc")];

            let mut app = custom_app::<CustomMsg, Empty, _>(|router, _, storage| {
                router
                    .bank
                    .init_balance(storage, &owner, init_funds)
                    .unwrap();
            });

            // set up reflect contract
            let code_id = app.store_code(reflect::contract());
            let init_msg = to_binary(&EmptyMsg {}).unwrap();
            let msg = WasmMsg::Instantiate {
                admin: None,
                code_id,
                msg: init_msg,
                funds: vec![],
                label: "label".into(),
            };
            let res = app.execute(owner, msg.into()).unwrap();

            // assert we have a proper instantiate result
            let parsed = parse_instantiate_response_data(res.data.unwrap().as_slice()).unwrap();
            assert!(parsed.data.is_none());
            // check the address is right

            let count: payout::CountResponse = app
                .wrap()
                .query_wasm_smart(&parsed.contract_address, &reflect::QueryMsg::Count {})
                .unwrap();
            assert_eq!(count.count, 0);
        }

        #[test]
        fn instantiate_with_data_works() {
            let owner = Addr::unchecked("owner");
            let mut app = BasicApp::new(|_, _, _| {});

            // set up echo contract
            let code_id = app.store_code(echo::contract());
            let msg = echo::InitMessage::<Empty> {
                data: Some("food".into()),
                sub_msg: None,
            };
            let init_msg = to_binary(&msg).unwrap();
            let msg = WasmMsg::Instantiate {
                admin: None,
                code_id,
                msg: init_msg,
                funds: vec![],
                label: "label".into(),
            };
            let res = app.execute(owner, msg.into()).unwrap();

            // assert we have a proper instantiate result
            let parsed = parse_instantiate_response_data(res.data.unwrap().as_slice()).unwrap();
            assert!(parsed.data.is_some());
            assert_eq!(parsed.data.unwrap(), Binary::from(b"food"));
            assert!(!parsed.contract_address.is_empty());
        }

        #[test]
        fn instantiate_with_reply_works() {
            let owner = Addr::unchecked("owner");
            let mut app = BasicApp::new(|_, _, _| {});

            // set up echo contract
            let code_id = app.store_code(echo::contract());
            let msg = echo::InitMessage::<Empty> {
                data: Some("food".into()),
                ..Default::default()
            };
            let addr1 = app
                .instantiate_contract(code_id, owner.clone(), &msg, &[], "first", None)
                .unwrap();

            // another echo contract
            let msg = echo::Message::<Empty> {
                data: Some("Passed to contract instantiation, returned as reply, and then returned as response".into()),
                ..Default::default()
            };
            let sub_msg = SubMsg::reply_on_success(
                WasmMsg::Execute {
                    contract_addr: addr1.to_string(),
                    msg: to_binary(&msg).unwrap(),
                    funds: vec![],
                },
                EXECUTE_REPLY_BASE_ID,
            );
            let init_msg = echo::InitMessage::<Empty> {
                data: Some("Overwrite me".into()),
                sub_msg: Some(vec![sub_msg]),
            };
            let init_msg = to_binary(&init_msg).unwrap();
            let msg = WasmMsg::Instantiate {
                admin: None,
                code_id,
                msg: init_msg,
                funds: vec![],
                label: "label".into(),
            };
            let res = app.execute(owner, msg.into()).unwrap();

            // assert we have a proper instantiate result
            let parsed = parse_instantiate_response_data(res.data.unwrap().as_slice()).unwrap();
            assert!(parsed.data.is_some());
            // Result is from the reply, not the original one
            assert_eq!(parsed.data.unwrap(), Binary::from(b"Passed to contract instantiation, returned as reply, and then returned as response"));
            assert!(!parsed.contract_address.is_empty());
            assert_ne!(parsed.contract_address, addr1.to_string());
        }

        #[test]
        fn execute_wrapped_properly() {
            let owner = Addr::unchecked("owner");
            let mut app = BasicApp::new(|_, _, _| {});

            // set up reflect contract
            let code_id = app.store_code(echo::contract());
            let echo_addr = app
                .instantiate_contract(code_id, owner.clone(), &EmptyMsg {}, &[], "label", None)
                .unwrap();

            // ensure the execute has the same wrapper as it should
            let msg = echo::Message::<Empty> {
                data: Some("hello".into()),
                ..echo::Message::default()
            };
            // execute_contract now decodes a protobuf wrapper, so we get the top-level response
            let exec_res = app.execute_contract(owner, echo_addr, &msg, &[]).unwrap();
            assert_eq!(exec_res.data, Some(Binary::from(b"hello")));
        }
    }

    mod errors {
        use super::*;

        #[test]
        fn simple_instantiation() {
            let owner = Addr::unchecked("owner");
            let mut app = App::default();

            // set up contract
            let code_id = app.store_code(error::contract(false));
            let msg = EmptyMsg {};
            let err = app
                .instantiate_contract(code_id, owner, &msg, &[], "error", None)
                .unwrap_err();

            // we should be able to retrieve the original error by downcasting
            let source: &StdError = err.downcast_ref().unwrap();
            if let StdError::GenericErr { msg } = source {
                assert_eq!(msg, "Init failed");
            } else {
                panic!("wrong StdError variant");
            }

            // We're expecting exactly 2 nested error types
            // (the original error, WasmMsg context)
            assert_eq!(err.chain().count(), 2);
        }

        #[test]
        fn simple_call() {
            let owner = Addr::unchecked("owner");
            let mut app = App::default();

            // set up contract
            let code_id = app.store_code(error::contract(true));
            let msg = EmptyMsg {};
            let contract_addr = app
                .instantiate_contract(code_id, owner, &msg, &[], "error", None)
                .unwrap();

            // execute should error
            let err = app
                .execute_contract(Addr::unchecked("random"), contract_addr, &msg, &[])
                .unwrap_err();

            // we should be able to retrieve the original error by downcasting
            let source: &StdError = err.downcast_ref().unwrap();
            if let StdError::GenericErr { msg } = source {
                assert_eq!(msg, "Handle failed");
            } else {
                panic!("wrong StdError variant");
            }

            // We're expecting exactly 2 nested error types
            // (the original error, WasmMsg context)
            assert_eq!(err.chain().count(), 2);
        }

        #[test]
        fn nested_call() {
            let owner = Addr::unchecked("owner");
            let mut app = App::default();

            let error_code_id = app.store_code(error::contract(true));
            let caller_code_id = app.store_code(caller::contract());

            // set up contracts
            let msg = EmptyMsg {};
            let caller_addr = app
                .instantiate_contract(caller_code_id, owner.clone(), &msg, &[], "caller", None)
                .unwrap();
            let error_addr = app
                .instantiate_contract(error_code_id, owner, &msg, &[], "error", None)
                .unwrap();

            // execute should error
            let msg = WasmMsg::Execute {
                contract_addr: error_addr.into(),
                msg: to_binary(&EmptyMsg {}).unwrap(),
                funds: vec![],
            };
            let err = app
                .execute_contract(Addr::unchecked("random"), caller_addr, &msg, &[])
                .unwrap_err();

            // we can downcast to get the original error
            let source: &StdError = err.downcast_ref().unwrap();
            if let StdError::GenericErr { msg } = source {
                assert_eq!(msg, "Handle failed");
            } else {
                panic!("wrong StdError variant");
            }

            // We're expecting exactly 3 nested error types
            // (the original error, 2 WasmMsg contexts)
            assert_eq!(err.chain().count(), 3);
        }

        #[test]
        fn double_nested_call() {
            let owner = Addr::unchecked("owner");
            let mut app = App::default();

            let error_code_id = app.store_code(error::contract(true));
            let caller_code_id = app.store_code(caller::contract());

            // set up contracts
            let msg = EmptyMsg {};
            let caller_addr1 = app
                .instantiate_contract(caller_code_id, owner.clone(), &msg, &[], "caller", None)
                .unwrap();
            let caller_addr2 = app
                .instantiate_contract(caller_code_id, owner.clone(), &msg, &[], "caller", None)
                .unwrap();
            let error_addr = app
                .instantiate_contract(error_code_id, owner, &msg, &[], "error", None)
                .unwrap();

            // caller1 calls caller2, caller2 calls error
            let msg = WasmMsg::Execute {
                contract_addr: caller_addr2.into(),
                msg: to_binary(&WasmMsg::Execute {
                    contract_addr: error_addr.into(),
                    msg: to_binary(&EmptyMsg {}).unwrap(),
                    funds: vec![],
                })
                .unwrap(),
                funds: vec![],
            };
            let err = app
                .execute_contract(Addr::unchecked("random"), caller_addr1, &msg, &[])
                .unwrap_err();

            // uncomment to have the test fail and see how the error stringifies
            // panic!("{:?}", err);

            // we can downcast to get the original error
            let source: &StdError = err.downcast_ref().unwrap();
            if let StdError::GenericErr { msg } = source {
                assert_eq!(msg, "Handle failed");
            } else {
                panic!("wrong StdError variant");
            }

            // We're expecting exactly 4 nested error types
            // (the original error, 3 WasmMsg contexts)
            assert_eq!(err.chain().count(), 4);
        }
    }
}
