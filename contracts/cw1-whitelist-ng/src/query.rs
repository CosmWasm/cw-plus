// This whole thing is definitely to be easly generated from trait itself

use std::borrow::Cow;

use cosmwasm_std::{Addr, CosmosMsg, CustomQuery, Empty, QuerierWrapper, StdResult};
use cw1::CanExecuteResponse;
use serde::Serialize;

use crate::msg::{AdminListResponse, QueryMsg};

// The `Cow` is used to provide ability to use it in two context:
// 1. Keeping owned as just a remote contract pointer - in such case we want it to keep ownership
//    over underlying address
// 2. Build ad-hoc temporarily from existing owned address, useful for querying contract via multiple
//    distinct queriers
pub struct Cw1WhitelistQuerier<'a>(Cow<'a, Addr>);

impl<'a> Cw1WhitelistQuerier<'static> {
    pub fn owned(addr: Addr) -> Self {
        Self(Cow::Owned(addr))
    }
}

impl<'a> Cw1WhitelistQuerier<'a> {
    pub fn new<'q, C>(
        addr: &'a Addr,
        querier: &'q QuerierWrapper<'q, C>,
    ) -> BoundCw1WhitelistQuerier<'a, 'q, C>
    where
        C: CustomQuery,
    {
        BoundCw1WhitelistQuerier { addr, querier }
    }

    pub fn bind<'q, C>(
        &self,
        querier: &'q QuerierWrapper<'q, C>,
    ) -> BoundCw1WhitelistQuerier<'_, 'q, C>
    where
        C: CustomQuery,
    {
        BoundCw1WhitelistQuerier {
            addr: &self.0,
            querier,
        }
    }
}

// Additional helper with already bound `QuerierWrapper`
pub struct BoundCw1WhitelistQuerier<'a, 'q, C>
where
    C: CustomQuery,
{
    addr: &'a Addr,
    querier: &'q QuerierWrapper<'q, C>,
}

impl<'a, 'q, C> BoundCw1WhitelistQuerier<'a, 'q, C>
where
    C: CustomQuery,
{
    pub fn admin_list(&self) -> StdResult<AdminListResponse> {
        self.querier
            .query_wasm_smart(self.addr.as_str(), &QueryMsg::<Empty>::AdminList {})
    }

    pub fn can_execute(
        &self,
        sender: String,
        msg: CosmosMsg<impl Serialize>,
    ) -> StdResult<CanExecuteResponse> {
        self.querier
            .query_wasm_smart(self.addr.as_str(), &QueryMsg::CanExecute { sender, msg })
    }
}
