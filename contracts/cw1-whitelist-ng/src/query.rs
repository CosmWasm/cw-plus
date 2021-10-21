// This whole thing is definitely to be easly generated from trait itself

use cosmwasm_std::{Addr, CosmosMsg, CustomQuery, QuerierWrapper, StdResult};
use cw1::CanExecuteResponse;
use serde::Serialize;

use crate::msg::{AdminListResponse, Cw1QueryMsg, WhitelistQueryMsg};

#[must_use]
pub struct Cw1Querier<'a, C>
where
    C: CustomQuery,
{
    addr: &'a Addr,
    querier: &'a QuerierWrapper<'a, C>,
}

impl<'a, C> Cw1Querier<'a, C>
where
    C: CustomQuery,
{
    pub fn new(addr: &'a Addr, querier: &'a QuerierWrapper<'a, C>) -> Self {
        Self { addr, querier }
    }

    pub fn can_execute(
        &self,
        sender: String,
        msg: CosmosMsg<impl Serialize>,
    ) -> StdResult<CanExecuteResponse> {
        self.querier
            .query_wasm_smart(self.addr.as_str(), &Cw1QueryMsg::CanExecute { sender, msg })
    }
}

#[must_use]
pub struct WhitelistQuerier<'a, C>
where
    C: CustomQuery,
{
    addr: &'a Addr,
    querier: &'a QuerierWrapper<'a, C>,
}

impl<'a, C> WhitelistQuerier<'a, C>
where
    C: CustomQuery,
{
    pub fn new(addr: &'a Addr, querier: &'a QuerierWrapper<'a, C>) -> Self {
        Self { addr, querier }
    }

    pub fn admin_list(&self) -> StdResult<AdminListResponse> {
        self.querier
            .query_wasm_smart(self.addr.as_str(), &WhitelistQueryMsg::AdminList {})
    }
}
