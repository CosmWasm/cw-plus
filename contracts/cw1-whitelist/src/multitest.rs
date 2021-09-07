use crate::msg::{AdminListResponse, ExecuteMsg, InstantiateMsg, QueryMsg};
use anyhow::{anyhow, Result};
use assert_matches::assert_matches;
use cosmwasm_std::testing::{mock_env, MockApi, MockStorage};
use cosmwasm_std::{to_binary, Addr, CosmosMsg, Empty, QueryRequest, WasmMsg, WasmQuery};
use cw1::Cw1Contract;
use cw_multi_test::{App, AppResponse, BankKeeper, Contract, ContractWrapper, Executor};
use derivative::Derivative;

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
    /// Sender of instantiated contract
    pub owner: Addr,
    /// Members of whitelist
    pub admins: Vec<Addr>,
    /// cw1 whitelist contract address
    pub whitelist: Cw1Contract,
}

impl Suite {
    pub fn init(ads: Vec<String>, mutable: bool) -> Result<Suite> {
        let mut app = mock_app();
        let owner = Addr::unchecked(ads[0].clone());
        let cw1_id = app.store_code(contract_cw1());

        let admins = ads
            .iter()
            .map(|address| Addr::unchecked(address))
            .collect::<Vec<Addr>>();

        let whitelist = app
            .instantiate_contract(
                cw1_id,
                owner.clone(),
                &InstantiateMsg {
                    admins: ads,
                    mutable,
                },
                &[],
                "Whitelist",
                None,
            )
            .unwrap();

        Ok(Suite {
            app,
            owner,
            admins,
            whitelist: Cw1Contract(whitelist),
        })
    }

    pub fn freeze(&mut self, sender: Addr, contract_addr: &Addr) -> Result<AppResponse> {
        let freeze_msg: ExecuteMsg = ExecuteMsg::Freeze {};
        let execute: ExecuteMsg = ExecuteMsg::Execute {
            msgs: vec![CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: contract_addr.to_string(),
                msg: to_binary(&freeze_msg)?,
                funds: vec![],
            })],
        };
        self.app
            .execute_contract(sender, self.whitelist.addr(), &execute, &[])
            .map_err(|err| anyhow!(err))
    }
}

#[test]
fn execute_freeze() {
    let owner = Addr::unchecked("owner");
    let mut suite = Suite::init(vec![owner.to_string()], true).unwrap();

    let cw1_id = suite.app.store_code(contract_cw1());
    let second_contract = suite
        .app
        .instantiate_contract(
            cw1_id,
            owner.clone(),
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
    assert_matches!(suite.freeze(owner.clone(), &second_contract), Ok(_));

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
                admins: _,
                mutable
            }) => {
            assert!(!mutable)
        }
    );
}
