use cosmwasm_std::{attr, Response, Uint128};
use cw_utils::Event;

/// Tracks token transfer/mint/burn actions
pub struct TransferEvent<'a> {
    pub from: Option<&'a str>,
    pub to: Option<&'a str>,
    pub token_id: &'a str,
    pub amount: Uint128,
}

impl<'a> Event for TransferEvent<'a> {
    fn add_attributes(&self, rsp: &mut Response) {
        rsp.attributes.push(attr("action", "transfer"));
        rsp.attributes.push(attr("token_id", self.token_id));
        rsp.attributes.push(attr("amount", self.amount));
        if let Some(from) = self.from {
            rsp.attributes.push(attr("from", from.to_string()));
        }
        if let Some(to) = self.to {
            rsp.attributes.push(attr("to", to.to_string()));
        }
    }
}

/// Tracks token metadata changes
pub struct MetadataEvent<'a> {
    pub url: &'a str,
    pub token_id: &'a str,
}

impl<'a> Event for MetadataEvent<'a> {
    fn add_attributes(&self, rsp: &mut Response) {
        rsp.attributes.push(attr("action", "set_metadata"));
        rsp.attributes.push(attr("url", self.url));
        rsp.attributes.push(attr("token_id", self.token_id));
    }
}

/// Tracks approve_all status changes
pub struct ApproveAllEvent<'a> {
    pub sender: &'a str,
    pub operator: &'a str,
    pub approved: bool,
}

impl<'a> Event for ApproveAllEvent<'a> {
    fn add_attributes(&self, rsp: &mut Response) {
        rsp.attributes.push(attr("action", "approve_all"));
        rsp.attributes.push(attr("sender", self.sender.to_string()));
        rsp.attributes
            .push(attr("operator", self.operator.to_string()));
        rsp.attributes
            .push(attr("approved", (self.approved as u32).to_string()));
    }
}
