use anyhow::{anyhow, Result};
use cosmwasm_std::testing::{mock_env, MockApi, MockStorage};
use cosmwasm_std::{to_binary, Addr, CosmosMsg, Empty, WasmMsg};
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
    /// Special account for performing administrative execution
    pub owner: Addr,
    /// Members of whitelist
    pub members: Vec<Addr>,
    /// cw1 whitelist contract address
    pub whitelist: Cw1Contract,
}

impl Suite {
    pub fn freeze(&mut self, addr: &Addr) -> Result<AppResponse> {
        let freeze_msg: crate::msg::ExecuteMsg = crate::msg::ExecuteMsg::Freeze {};
        let execute: crate::msg::ExecuteMsg = crate::msg::ExecuteMsg::Execute {
            msgs: vec![CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: addr.to_string(),
                msg: to_binary(&freeze_msg)?,
                funds: vec![],
            })],
        };
        self.app
            .execute_contract(self.owner.clone(), self.whitelist.addr(), &execute, &[])
            .map_err(|err| anyhow!(err))
    }
}

#[derive(Default)]
pub struct Config {
    /// Initial members
    members: Vec<Addr>,
}

impl Config {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_member(mut self, addr: &str) -> Self {
        self.members.push(Addr::unchecked(addr));

        self
    }

    pub fn init(self, admins: Vec<String>, mutable: bool) -> Result<Suite> {
        let mut app = mock_app();
        let owner = Addr::unchecked(admins[0].clone());
        let cw1_id = app.store_code(contract_cw1());

        let members: Vec<_> = self
            .members
            .into_iter()
            .map(|address| -> Result<_> {
                let member = address.to_string();
                Ok(member)
            })
            .collect::<Result<Vec<_>>>()?;

        let whitelist = app
            .instantiate_contract(
                cw1_id,
                owner.clone(),
                &crate::msg::InstantiateMsg { admins, mutable },
                &[],
                "Whitelist",
                None,
            )
            .unwrap();

        let members = members
            .into_iter()
            .map(|address| Addr::unchecked(address))
            .collect();

        Ok(Suite {
            app,
            owner,
            members,
            whitelist: Cw1Contract(whitelist),
        })
    }
}

#[test]
fn execute_freeze() {
    let mut suite1 = Config::new()
        .with_member("member1")
        .init(vec!["member1".to_owned()], true)
        .unwrap();

    let mut suite2 = Config::new()
        .with_member("member2")
        .init(vec!["member2".to_owned()], true)
        .unwrap();

    let member = suite1.members[0].clone();

    let err_freeze = suite1.freeze(&suite2.members[0]).unwrap_err();
}
