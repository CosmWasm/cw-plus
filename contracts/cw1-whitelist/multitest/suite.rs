use cosmwasm_std::testing::{mock_env, MockApi, MockStorage};
use cosmwasm_std::{to_binary, Addr, Binary, Empty, Response, StdError, Uint128};
use cw_multi_test::{App, AppResponse, BankKeeper, Contract, ContractWrapper, Executor};

fn mock_app() -> App {
    let env = mock_env();
    let api = MockApi::default();
    let bank = BankKeeper::new();
    let storage = MockStorage::new();

    App::new(api, env.block, bank, storage)
}

