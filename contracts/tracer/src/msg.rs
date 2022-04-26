use crate::state::LogEntry;
use cosmwasm_std::Binary;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Touch {},
    Fail {},
    Forward {
        addr: String,
        msg: Binary,
        marker: u64,
        catch_success: bool,
        catch_failure: bool,
        fail_reply: bool,
    },
    Clear {},
    Reset {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Log { depth: Option<usize> },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct LogResponse {
    pub log: Vec<Vec<LogEntry>>,
}
