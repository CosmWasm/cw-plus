use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{to_binary, Binary, CosmosMsg, HumanAddr, StdResult, WasmMsg};

/// MemberChangedHookMsg should be de/serialized under `MemberChangedHook()` variant in a HandleMsg
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub struct MemberChangedHookMsg {
    pub diffs: Vec<MemberDiff>,
}

/// MemberDiff shows the old and new states for a given cw4 member
/// They cannot both be None.
/// old = None, new = Some -> Insert
/// old = Some, new = Some -> Update
/// old = Some, new = None -> Delete
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct MemberDiff {
    pub addr: HumanAddr,
    pub old_weight: Option<u64>,
    pub new_weight: Option<u64>,
}

impl MemberDiff {
    pub fn new<T: Into<HumanAddr>>(
        addr: T,
        old_weight: Option<u64>,
        new_weight: Option<u64>,
    ) -> Self {
        MemberDiff {
            addr: addr.into(),
            old_weight,
            new_weight,
        }
    }
}

impl MemberChangedHookMsg {
    /// serializes the message
    pub fn into_binary(self) -> StdResult<Binary> {
        let msg = MemberChangedHandleMsg::MemberChangedHook(self);
        to_binary(&msg)
    }

    /// creates a cosmos_msg sending this struct to the named contract
    pub fn into_cosmos_msg(self, contract_addr: HumanAddr) -> StdResult<CosmosMsg> {
        let msg = self.into_binary()?;
        let execute = WasmMsg::Execute {
            contract_addr,
            msg,
            send: vec![],
        };
        Ok(execute.into())
    }
}

// This is just a helper to properly serialize the above message
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
enum MemberChangedHandleMsg {
    MemberChangedHook(MemberChangedHookMsg),
}
