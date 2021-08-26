use std::fmt;

use crate::parse_contract_addr;
use cosmwasm_std::{
    to_binary, Addr, Attribute, BankMsg, Binary, Coin, CosmosMsg, Event, SubMsgExecutionResponse,
    WasmMsg,
};
use schemars::JsonSchema;
use serde::Serialize;

use anyhow::Result as AnyResult;

#[derive(Default, Clone, Debug)]
pub struct AppResponse {
    pub events: Vec<Event>,
    pub data: Option<Binary>,
}

impl AppResponse {
    // Return all custom attributes returned by the contract in the `idx` event.
    // We assert the type is wasm, and skip the contract_address attribute.
    #[track_caller]
    pub fn custom_attrs(&self, idx: usize) -> &[Attribute] {
        assert_eq!(self.events[idx].ty.as_str(), "wasm");
        &self.events[idx].attributes[1..]
    }

    /// Check if there is an Event that is a super-set of this.
    /// It has the same type, and all compare.attributes are included in it as well.
    /// You don't need to specify them all.
    pub fn has_event(&self, expected: &Event) -> bool {
        self.events.iter().any(|ev| {
            expected.ty == ev.ty
                && expected
                    .attributes
                    .iter()
                    .all(|at| ev.attributes.contains(at))
        })
    }

    /// Like has_event but panics if no match
    #[track_caller]
    pub fn assert_event(&self, expected: &Event) {
        assert!(
            self.has_event(expected),
            "Expected to find an event {:?}, but received: {:?}",
            expected,
            self.events
        );
    }
}

/// They have the same shape, SubMsgExecutionResponse is what is returned in reply.
/// This is just to make some test cases easier.
impl From<SubMsgExecutionResponse> for AppResponse {
    fn from(reply: SubMsgExecutionResponse) -> Self {
        AppResponse {
            data: reply.data,
            events: reply.events,
        }
    }
}

pub trait Executor<C>
where
    C: Clone + fmt::Debug + PartialEq + JsonSchema + 'static,
{
    /// Runs arbitrary CosmosMsg.
    /// This will create a cache before the execution, so no state changes are persisted if this
    /// returns an error, but all are persisted on success.
    fn execute(&mut self, sender: Addr, msg: CosmosMsg<C>) -> AnyResult<AppResponse>;

    /// Create a contract and get the new address.
    /// This is just a helper around execute()
    fn instantiate_contract<T: Serialize, U: Into<String>>(
        &mut self,
        code_id: u64,
        sender: Addr,
        init_msg: &T,
        send_funds: &[Coin],
        label: U,
        admin: Option<String>,
    ) -> AnyResult<Addr> {
        // instantiate contract
        let init_msg = to_binary(init_msg)?;
        let msg = WasmMsg::Instantiate {
            admin,
            code_id,
            msg: init_msg,
            funds: send_funds.to_vec(),
            label: label.into(),
        };
        let res = self.execute(sender, msg.into())?;
        parse_contract_addr(&res.data)
    }

    /// Execute a contract and process all returned messages.
    /// This is just a helper around execute()
    fn execute_contract<T: Serialize>(
        &mut self,
        sender: Addr,
        contract_addr: Addr,
        msg: &T,
        send_funds: &[Coin],
    ) -> AnyResult<AppResponse> {
        let msg = to_binary(msg)?;
        let msg = WasmMsg::Execute {
            contract_addr: contract_addr.into(),
            msg,
            funds: send_funds.to_vec(),
        };
        self.execute(sender, msg.into())
    }

    /// Migrate a contract. Sender must be registered admin.
    /// This is just a helper around execute()
    fn migrate_contract<T: Serialize>(
        &mut self,
        sender: Addr,
        contract_addr: Addr,
        msg: &T,
        new_code_id: u64,
    ) -> AnyResult<AppResponse> {
        let msg = to_binary(msg)?;
        let msg = WasmMsg::Migrate {
            contract_addr: contract_addr.into(),
            msg,
            new_code_id,
        };
        self.execute(sender, msg.into())
    }

    fn send_tokens(
        &mut self,
        sender: Addr,
        recipient: Addr,
        amount: &[Coin],
    ) -> AnyResult<AppResponse> {
        let msg = BankMsg::Send {
            to_address: recipient.to_string(),
            amount: amount.to_vec(),
        };
        self.execute(sender, msg.into())
    }
}
