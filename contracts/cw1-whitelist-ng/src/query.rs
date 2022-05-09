// This whole thing is definitely to be easly generated from trait itself

use cosmwasm_std::{Addr, CosmosMsg, QuerierWrapper, StdResult};
use cw1::CanExecuteResponse;
use serde::Serialize;

use crate::msg::{self, AdminListResponse};

#[must_use]
pub struct Cw1Querier<'a> {
    addr: &'a Addr,
    querier: &'a QuerierWrapper<'a>,
}

impl<'a> Cw1Querier<'a> {
    pub fn new(addr: &'a Addr, querier: &'a QuerierWrapper<'a>) -> Self {
        Self { addr, querier }
    }

    pub fn can_execute(
        &self,
        sender: String,
        msg: CosmosMsg<impl Serialize + std::fmt::Debug + PartialEq + Clone + schemars::JsonSchema>,
    ) -> StdResult<CanExecuteResponse> {
        self.querier.query_wasm_smart(
            self.addr.as_str(),
            &msg::cw1_msg::QueryMsg::CanExecute { sender, msg },
        )
    }
}

#[must_use]
pub struct WhitelistQuerier<'a> {
    addr: &'a Addr,
    querier: &'a QuerierWrapper<'a>,
}

impl<'a> WhitelistQuerier<'a> {
    pub fn new(addr: &'a Addr, querier: &'a QuerierWrapper<'a>) -> Self {
        Self { addr, querier }
    }

    pub fn admin_list(&self) -> StdResult<AdminListResponse> {
        self.querier
            .query_wasm_smart(self.addr.as_str(), &msg::whitelist::QueryMsg::AdminList {})
    }
}
