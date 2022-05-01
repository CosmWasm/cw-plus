use anyhow::Result as AnyResult;
use cosmwasm_std::{to_binary, Addr, Empty};
use cw_multi_test::{App, AppResponse, Contract, ContractWrapper, Executor};
use derivative::Derivative;
use serde::Serialize;

use crate::msg::{ExecuteMsg, LogResponse, QueryMsg};
use crate::state::LogEntry;

fn contract_tracer() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    )
    .with_reply(crate::contract::reply);
    Box::new(contract)
}

pub struct SuiteBuilder;

impl SuiteBuilder {
    pub fn new() -> Self {
        Self
    }

    #[track_caller]
    pub fn build(self) -> Suite {
        let mut app = App::default();
        let owner = "owner".to_owned();
        let contract_id = app.store_code(contract_tracer());

        let contract = app
            .instantiate_contract(
                contract_id,
                Addr::unchecked(owner),
                &Empty {},
                &[],
                "Tracer",
                None,
            )
            .unwrap();

        Suite { app, contract }
    }
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct Suite {
    #[derivative(Debug = "ignore")]
    app: App,
    contract: Addr,
}

impl Suite {
    pub fn new() -> Self {
        SuiteBuilder::new().build()
    }

    pub fn contract_addr(&self) -> String {
        self.contract.as_str().to_owned()
    }

    pub fn touch(&mut self, sender: &str) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.contract.clone(),
            &ExecuteMsg::Touch {},
            &[],
        )
    }

    pub fn fail(&mut self, sender: &str) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.contract.clone(),
            &ExecuteMsg::Fail {},
            &[],
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn forward(
        &mut self,
        sender: &str,
        addr: &str,
        msg: impl Serialize,
        marker: u64,
        catch_success: bool,
        catch_failure: bool,
        fail_reply: bool,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.contract.clone(),
            &ExecuteMsg::Forward {
                addr: addr.to_owned(),
                msg: to_binary(&msg)?,
                marker,
                catch_success,
                catch_failure,
                fail_reply,
            },
            &[],
        )
    }

    pub fn clear(&mut self, sender: &str) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.contract.clone(),
            &ExecuteMsg::Clear {},
            &[],
        )
    }

    pub fn reset(&mut self, sender: &str) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.contract.clone(),
            &ExecuteMsg::Reset {},
            &[],
        )
    }

    pub fn log(&self, depth: impl Into<Option<u32>>) -> AnyResult<Vec<Vec<LogEntry>>> {
        let resp: LogResponse = self.app.wrap().query_wasm_smart(
            self.contract.clone(),
            &QueryMsg::Log {
                depth: depth.into(),
            },
        )?;

        Ok(resp.log)
    }
}
