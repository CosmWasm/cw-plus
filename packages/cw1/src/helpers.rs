use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{to_binary, Addr, Api, CanonicalAddr, CosmosMsg, StdResult, WasmMsg};

use crate::msg::Cw1ExecuteMsg;

/// Cw1Contract is a wrapper around Addr that provides a lot of helpers
/// for working with this.
///
/// If you wish to persist this, convert to Cw1CanonicalContract via .canonical()
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Cw1Contract(pub Addr);

impl Cw1Contract {
    pub fn addr(&self) -> Addr {
        self.0.clone()
    }

    /// Convert this address to a form fit for storage
    pub fn canonical<A: Api>(&self, api: &A) -> StdResult<Cw1CanonicalContract> {
        let canon = api.addr_canonicalize(self.0.as_ref())?;
        Ok(Cw1CanonicalContract(canon))
    }

    pub fn execute<T: Into<Vec<CosmosMsg>>>(&self, msgs: T) -> StdResult<CosmosMsg> {
        let msg = Cw1ExecuteMsg::Execute { msgs: msgs.into() };
        Ok(WasmMsg::Execute {
            contract_addr: self.addr().into(),
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
        let human = api.addr_humanize(&self.0)?;
        Ok(Cw1Contract(human))
    }
}
