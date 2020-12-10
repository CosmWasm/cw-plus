use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::HumanAddr;

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Cw4HandleMsg {
    /// Change the admin
    UpdateAdmin { admin: Option<HumanAddr> },
    /// Add a new hook to be informed of all membership changes. Must be called by Admin
    AddHook { addr: HumanAddr },
    /// Remove a hook. Must be called by Admin
    RemoveHook { addr: HumanAddr },
}
