use crate::msg::{AdminListResponse, ExecuteMsg, InstantiateMsg, QueryMsg};
use anyhow::{anyhow, Result};
use assert_matches::assert_matches;
use cosmwasm_std::{to_binary, Addr, CosmosMsg, Empty, QueryRequest, StdError, WasmMsg, WasmQuery};
use cw1::Cw1Contract;
use cw_multi_test::{App, AppResponse, Contract, ContractWrapper, Executor};
use derivative::Derivative;
use serde::{de::DeserializeOwned, Serialize};

fn mock_app() -> App {
    App::default()
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
    app: App,
    /// Special account
    pub owner: String,
    /// ID of stored code for cw1 contract
    cw1_id: u64,
}

impl Suite {
    pub fn init() -> Result<Suite> {
        let mut app = mock_app();
        let owner = "owner".to_owned();
        let cw1_id = app.store_code(contract_cw1());

        Ok(Suite { app, owner, cw1_id })
    }

    pub fn instantiate_cw1_contract(&mut self, admins: Vec<String>, mutable: bool) -> Cw1Contract {
        let contract = self
            .app
            .instantiate_contract(
                self.cw1_id,
                Addr::unchecked(self.owner.clone()),
                &InstantiateMsg { admins, mutable },
                &[],
                "Whitelist",
                None,
            )
            .unwrap();
        Cw1Contract(contract)
    }

    pub fn execute<M>(
        &mut self,
        sender_contract: Addr,
        target_contract: &Addr,
        msg: M,
    ) -> Result<AppResponse>
    where
        M: Serialize + DeserializeOwned,
    {
        let execute: ExecuteMsg = ExecuteMsg::Execute {
            msgs: vec![CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: target_contract.to_string(),
                msg: to_binary(&msg)?,
                funds: vec![],
            })],
        };
        self.app
            .execute_contract(
                Addr::unchecked(self.owner.clone()),
                sender_contract,
                &execute,
                &[],
            )
            .map_err(|err| anyhow!(err))
    }

    pub fn query<M>(&self, target_contract: Addr, msg: M) -> Result<AdminListResponse, StdError>
    where
        M: Serialize + DeserializeOwned,
    {
        self.app.wrap().query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: target_contract.to_string(),
            msg: to_binary(&msg).unwrap(),
        }))
    }
}

#[test]
fn proxy_freeze_message() {
    let mut suite = Suite::init().unwrap();

    let first_contract = suite.instantiate_cw1_contract(vec![suite.owner.clone()], true);
    let second_contract =
        suite.instantiate_cw1_contract(vec![first_contract.addr().to_string()], true);
    assert_ne!(second_contract, first_contract);

    let freeze_msg: ExecuteMsg = ExecuteMsg::Freeze {};
    assert_matches!(
        suite.execute(first_contract.addr(), &second_contract.addr(), freeze_msg),
        Ok(_)
    );

    let query_msg: QueryMsg = QueryMsg::AdminList {};
    assert_matches!(
        suite.query(second_contract.addr(), query_msg),
        Ok(
            AdminListResponse {
                mutable,
                ..
            }) if !mutable
    );
}
