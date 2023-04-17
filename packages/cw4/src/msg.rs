#[cfg(features="boot")]
use boot_fns_derive::ExecuteFns;
use cosmwasm_schema::cw_serde;

#[cw_serde]
#[cfg_attr(features="boot", derive(ExecuteFns))]
pub enum Cw4ExecuteMsg {
    /// Change the admin
    UpdateAdmin { admin: Option<String> },
    /// Add a new hook to be informed of all membership changes. Must be called by Admin
    AddHook { addr: String },
    /// Remove a hook. Must be called by Admin
    RemoveHook { addr: String },
}
