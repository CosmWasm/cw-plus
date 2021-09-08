use crate::msg::{AdminListResponse, ExecuteMsg, InstantiateMsg, QueryMsg};
use anyhow::{anyhow, Result};
use assert_matches::assert_matches;
use cosmwasm_std::testing::{mock_env, MockApi, MockStorage};
use cosmwasm_std::{to_binary, Addr, CosmosMsg, Empty, QueryRequest, WasmMsg, WasmQuery};
use cw1::Cw1Contract;
use cw_multi_test::{App, AppResponse, BankKeeper, Contract, ContractWrapper, Executor};
use derivative::Derivative;
use serde::{de::DeserializeOwned, Serialize};

fn mock_app() -> App {
    let env = mock_env();
    let api = MockApi::default();
    let bank = BankKeeper::new();
    let storage = MockStorage::new();

    App::new(api, env.block, bank, storage)
}

fn contract_cw1() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    );
    Box::new(contract)
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct Suite {
    /// Application mock
    #[derivative(Debug = "ignore")]
    pub app: App,
    /// cw1 whitelist contract address
    pub whitelist: Cw1Contract,
    /// Special account
    pub owner: String,
}

impl Suite {
    pub fn init(mutable: bool) -> Result<Suite> {
        let mut app = mock_app();
        let cw1_id = app.store_code(contract_cw1());
        let owner = "owner".to_owned();

        let whitelist = app
            .instantiate_contract(
                cw1_id,
                Addr::unchecked(owner.clone()),
                &InstantiateMsg {
                    admins: vec![owner.clone()],
                    mutable,
                },
                &[],
                "Whitelist",
                None,
            )
            .unwrap();

        Ok(Suite {
            app,
            whitelist: Cw1Contract(whitelist),
            owner,
        })
    }

    pub fn execute<M>(&mut self, contract_addr: &Addr, msg: M) -> Result<AppResponse>
    where
        M: Serialize + DeserializeOwned,
    {
        let execute: ExecuteMsg = ExecuteMsg::Execute {
            msgs: vec![CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: contract_addr.to_string(),
                msg: to_binary(&msg)?,
                funds: vec![],
            })],
        };
        self.app
            .execute_contract(
                Addr::unchecked(self.owner.clone()),
                self.whitelist.addr(),
                &execute,
                &[],
            )
            .map_err(|err| anyhow!(err))
    }
}

#[test]
fn proxy_freeze_message() {
    let mut suite = Suite::init(true).unwrap();

    let owner = "owner".to_string();
    let cw1_id = suite.app.store_code(contract_cw1());
    let second_contract = suite
        .app
        .instantiate_contract(
            cw1_id,
            Addr::unchecked(owner),
            &InstantiateMsg {
                admins: vec![suite.whitelist.0.to_string()],
                mutable: true,
            },
            &[],
            "Whitelist",
            None,
        )
        .unwrap();

    assert_ne!(second_contract, suite.whitelist.0);
    let freeze_msg: ExecuteMsg = ExecuteMsg::Freeze {};
    assert_matches!(suite.execute(&second_contract, freeze_msg), Ok(_));

    let query_msg: QueryMsg = QueryMsg::AdminList {};
    assert_matches!(
        suite
            .app
            .wrap()
            .query(&QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: second_contract.to_string(),
                msg: to_binary(&query_msg).unwrap(),
            })
        ),
        Ok(
            AdminListResponse {
                mutable,
                ..
            }) => {
            assert!(!mutable)
        }
    );
}
