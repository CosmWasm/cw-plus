use cosmwasm_std::{attr, Attribute, HumanAddr, Uint128};
use cw0::Event;

use crate::msg::TokenId;

pub struct TransferEvent<'a> {
    pub from: Option<&'a HumanAddr>,
    pub to: Option<&'a HumanAddr>,
    pub token_id: TokenId,
    pub amount: Uint128,
}

impl Event for TransferEvent {
    fn write_attributes(&self, attributes: &mut Vec<Attribute>) {
        attributes.extend_from_slice(&[
            attr("action", "transfer"),
            attr("token_id", self.token_id),
            attr("amount", self.amount),
        ]);
        if let Some(from) = from {
            attributes.push(attr("from", from));
        }
        if let Some(to) = to {
            attributes.push(attr("to", to));
        }
    }
}

pub struct MetadataEvent<'a> {
    pub url: &'a str,
    pub token_id: TokenId,
}

impl Event for URLEvent {
    fn write_attributes(&self, attributes: &mut Vec<Attribute>) {
        attributes.extend_from_slice(&[
            attr("action", "set_metadata"),
            attr("url", self.url),
            attr("token_id", self.token_id),
        ]);
    }
}
