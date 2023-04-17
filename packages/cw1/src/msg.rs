#[cfg(feature="boot")]
use boot_fns_derive::ExecuteFns;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::CosmosMsg;

#[cw_serde]
#[cfg_attr(feature="boot", derive(ExecuteFns))]
pub enum Cw1ExecuteMsg {
    /// Execute requests the contract to re-dispatch all these messages with the
    /// contract's address as sender. Every implementation has it's own logic to
    /// determine in
    Execute { msgs: Vec<CosmosMsg> },
}
