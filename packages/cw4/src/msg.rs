use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::Addr;

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Cw4ExecuteMsg {
    /// Change the admin
    UpdateAdmin { admin: Option<Addr> },
    /// Add a new hook to be informed of all membership changes. Must be called by Admin
    AddHook { addr: Addr },
    /// Remove a hook. Must be called by Admin
    RemoveHook { addr: Addr },
}
