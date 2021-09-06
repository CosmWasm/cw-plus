use cosmwasm_std::testing::{mock_env, MockApi, MockStorage};
use cw_multi_test::{App, BankKeeper, Contract, ContractWrapper, Executor};
use cosmwasm_std::{Addr, Empty};
use anyhow::Result;
use cw1::Cw1Contract;
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

/// Configuration of single member
struct MemberConfig {
    /// Member address
    addr: String,
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

#[derive(Default)]
pub struct Config {
    /// Initial members
    members: Vec<MemberConfig>,
}

impl Config {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_member(mut self, addr: &str) -> Self {
        self.members.push(MemberConfig {
            addr: addr.to_owned(),
        });

        self
    }

        pub fn init(self, admins: Vec<String>) -> Result<Suite> {
        let mut app = mock_app();
        let owner = Addr::unchecked("owner");
        let cw1_id = app.store_code(contract_cw1());

        let members: Vec<_> = self
            .members
            .into_iter()
            .map(|member| -> Result<_> {
                let member = MemberConfig {
                    addr: member.addr.to_string(),
                };
                Ok(member)
            })
            .collect::<Result<Vec<_>>>()?;

        let whitelist = app
            .instantiate_contract(
                cw1_id,
                owner.clone(),
                &crate::msg::InstantiateMsg {
                    admins,
                    mutable: false,
                },
                &[],
                "Whitelist",
                None,
            )
            .unwrap();

        let members = members
            .into_iter()
            .map(|member| Addr::unchecked(member.addr))
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
    let _suite = Config::new()
        .with_member("member1")
        .with_member("member2")
        .init(vec!["member1".to_owned()])
        .unwrap();
}
