#[cfg(feature="boot")]
use boot_core::QueryFns;

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::CosmosMsg;

#[cw_serde]
#[derive(QueryResponses)]
#[cfg_attr(feature="boot", derive(QueryFns))]
pub enum Cw1QueryMsg {
    /// Checks permissions of the caller on this proxy.
    /// If CanExecute returns true then a call to `Execute` with the same message,
    /// from the given sender, before any further state changes, should also succeed.
    #[returns(CanExecuteResponse)]
    CanExecute { sender: String, msg: CosmosMsg },
}

#[cw_serde]
pub struct CanExecuteResponse {
    pub can_execute: bool,
}
