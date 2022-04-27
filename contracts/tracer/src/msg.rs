use cosmwasm_std::Binary;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::state::LogEntry;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Touch {},
    Fail {},
    Forward {
        addr: String,
        msg: Binary,
        #[serde(default)]
        marker: u64,
        #[serde(default)]
        catch_success: bool,
        #[serde(default)]
        catch_failure: bool,
        #[serde(default)]
        fail_reply: bool,
    },
    Clear {},
    Reset {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Log { depth: Option<u32> },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct LogResponse {
    pub log: Vec<Vec<LogEntry>>,
}
