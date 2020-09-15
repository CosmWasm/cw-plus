use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;

use cosmwasm_std::{CosmosMsg, Empty, HumanAddr};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Cw1QueryMsg<T = Empty>
where
    T: Clone + fmt::Debug + PartialEq + JsonSchema,
{
    /// Checks permissions of the caller on this proxy.
    /// If CanSend returns true then a call to `Execute` with the same message,
    /// from the given sender, before any further state changes, should also succeed.
    CanSend {
        sender: HumanAddr,
        msg: CosmosMsg<T>,
    },
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct CanSendResponse {
    pub can_send: bool,
}
