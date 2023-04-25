#[cfg(feature="interface")]
use cw_orchestrate::ExecuteFns;

use cosmwasm_std::Empty;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::CosmosMsg;

#[cw_serde]
#[cfg_attr(feature="interface", derive(ExecuteFns))]
pub enum Cw1ExecuteMsg<T = Empty> {
    /// Execute requests the contract to re-dispatch all these messages with the
    /// contract's address as sender. Every implementation has it's own logic to
    /// determine in
    Execute { msgs: Vec<CosmosMsg<T>> },
}
