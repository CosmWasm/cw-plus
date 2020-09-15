use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{to_binary, Api, CanonicalAddr, CosmosMsg, HumanAddr, StdResult, WasmMsg};

use crate::msg::Cw1HandleMsg;

/// Cw1Contract is a wrapper around HumanAddr that provides a lot of helpers
/// for working with this.
///
/// If you wish to persist this, convert to Cw1CanonicalContract via .canonical()
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Cw1Contract(pub HumanAddr);

impl Cw1Contract {
    pub fn addr(&self) -> HumanAddr {
        self.0.clone()
    }

    /// Convert this address to a form fit for storage
    pub fn canonical<A: Api>(&self, api: &A) -> StdResult<Cw1CanonicalContract> {
        let canon = api.canonical_address(&self.0)?;
        Ok(Cw1CanonicalContract(canon))
    }

    pub fn execute<T: Into<Vec<CosmosMsg>>>(&self, msgs: T) -> StdResult<CosmosMsg> {
        let msg = Cw1HandleMsg::Execute { msgs: msgs.into() };
        Ok(WasmMsg::Execute {
            contract_addr: self.addr(),
            msg: to_binary(&msg)?,
            send: vec![],
        }
        .into())
    }
}

/// This is a respresentation of Cw1Contract for storage.
/// Don't use it directly, just translate to the Cw1Contract when needed.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Cw1CanonicalContract(pub CanonicalAddr);

impl Cw1CanonicalContract {
    /// Convert this address to a form fit for usage in messages and queries
    pub fn human<A: Api>(&self, api: &A) -> StdResult<Cw1Contract> {
        let human = api.human_address(&self.0)?;
        Ok(Cw1Contract(human))
    }
}
