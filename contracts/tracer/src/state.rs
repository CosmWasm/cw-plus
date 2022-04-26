use cosmwasm_std::Addr;
use cw_storage_plus::Item;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::msg::ExecuteMsg;

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub struct LogEntry {
    pub sender: Addr,
    pub msg: ExecuteMsg,
    pub reply: bool,
    pub marker: Option<u64>,
}

// Persistent actions log
pub const LOG: Item<Vec<Vec<LogEntry>>> = Item::new("log");

// Message being processed, for logging on reply purposes
pub const PROCESSED_MSG: Item<ExecuteMsg> = Item::new("processed");
