use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::ops::Deref;

use cosmwasm_std::{to_binary, Api, CanonicalAddr, CosmosMsg, HumanAddr, StdResult, WasmMsg};
use cw4::{Cw4Contract, Member};

use crate::msg::HandleMsg;

/// Cw4GroupContract is a wrapper around HumanAddr that provides a lot of helpers
/// for working with cw4-group contracts.
///
/// It extends Cw4Contract to add the extra calls from cw4-group.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Cw4GroupContract(pub Cw4Contract);

impl Deref for Cw4GroupContract {
    type Target = Cw4Contract;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Cw4GroupContract {
    pub fn new(addr: HumanAddr) -> Self {
        Cw4GroupContract(Cw4Contract(addr))
    }

    /// Convert this address to a form fit for storage
    pub fn canonical(&self, api: &dyn Api) -> StdResult<Cw4GroupCanonicalContract> {
        let canon = api.canonical_address(&self.addr())?;
        Ok(Cw4GroupCanonicalContract(canon))
    }

    fn encode_msg(&self, msg: HandleMsg) -> StdResult<CosmosMsg> {
        Ok(WasmMsg::Execute {
            contract_addr: self.addr(),
            msg: to_binary(&msg)?,
            send: vec![],
        }
        .into())
    }

    pub fn update_members(&self, remove: Vec<HumanAddr>, add: Vec<Member>) -> StdResult<CosmosMsg> {
        let msg = HandleMsg::UpdateMembers { remove, add };
        self.encode_msg(msg)
    }
}

/// This is a representation of Cw4GroupContract for storage.
/// Don't use it directly, just translate to the Cw4GroupContract when needed.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Cw4GroupCanonicalContract(pub CanonicalAddr);

impl Cw4GroupCanonicalContract {
    /// Convert this address to a form fit for usage in messages and queries
    pub fn human(&self, api: &dyn Api) -> StdResult<Cw4GroupContract> {
        let human = api.human_address(&self.0)?;
        Ok(Cw4GroupContract::new(human))
    }
}
