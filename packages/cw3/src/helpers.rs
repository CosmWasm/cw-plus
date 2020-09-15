use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{to_binary, Api, CanonicalAddr, CosmosMsg, HumanAddr, StdResult, WasmMsg};

use crate::msg::{Cw3HandleMsg, Vote};
use cw0::Expiration;

/// Cw3Contract is a wrapper around HumanAddr that provides a lot of helpers
/// for working with this.
///
/// If you wish to persist this, convert to Cw3CanonicalContract via .canonical()
///
/// FIXME: Cw3Contract currently only supports CosmosMsg<Empty>. When we actually
/// use this in some consuming code, we should make it generic over CosmosMsg<T>.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Cw3Contract(pub HumanAddr);

impl Cw3Contract {
    pub fn addr(&self) -> HumanAddr {
        self.0.clone()
    }

    /// Convert this address to a form fit for storage
    pub fn canonical<A: Api>(&self, api: &A) -> StdResult<Cw3CanonicalContract> {
        let canon = api.canonical_address(&self.0)?;
        Ok(Cw3CanonicalContract(canon))
    }

    pub fn encode_msg(&self, msg: Cw3HandleMsg) -> StdResult<CosmosMsg> {
        Ok(WasmMsg::Execute {
            contract_addr: self.addr(),
            msg: to_binary(&msg)?,
            send: vec![],
        }
        .into())
    }

    /// helper doesn't support custom messages now
    pub fn proposal<T: Into<String>, U: Into<String>>(
        &self,
        title: T,
        description: U,
        msgs: Vec<CosmosMsg>,
        earliest: Option<Expiration>,
        latest: Option<Expiration>,
    ) -> StdResult<CosmosMsg> {
        let msg = Cw3HandleMsg::Propose {
            title: title.into(),
            description: description.into(),
            msgs,
            earliest,
            latest,
        };
        self.encode_msg(msg)
    }

    pub fn vote(&self, proposal_id: u64, vote: Vote) -> StdResult<CosmosMsg> {
        let msg = Cw3HandleMsg::Vote { proposal_id, vote };
        self.encode_msg(msg)
    }

    pub fn execute(&self, proposal_id: u64) -> StdResult<CosmosMsg> {
        let msg = Cw3HandleMsg::Execute { proposal_id };
        self.encode_msg(msg)
    }

    pub fn close(&self, proposal_id: u64) -> StdResult<CosmosMsg> {
        let msg = Cw3HandleMsg::Close { proposal_id };
        self.encode_msg(msg)
    }
}

/// This is a respresentation of Cw3Contract for storage.
/// Don't use it directly, just translate to the Cw3Contract when needed.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Cw3CanonicalContract(pub CanonicalAddr);

impl Cw3CanonicalContract {
    /// Convert this address to a form fit for usage in messages and queries
    pub fn human<A: Api>(&self, api: &A) -> StdResult<Cw3Contract> {
        let human = api.human_address(&self.0)?;
        Ok(Cw3Contract(human))
    }
}
