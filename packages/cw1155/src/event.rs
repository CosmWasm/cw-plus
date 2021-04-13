use cosmwasm_std::{Response, Uint128};
use cw0::Event;

/// Tracks token transfer/mint/burn actions
pub struct TransferEvent<'a> {
    pub from: Option<&'a str>,
    pub to: Option<&'a str>,
    pub token_id: &'a str,
    pub amount: Uint128,
}

impl<'a> Event for TransferEvent<'a> {
    fn add_attributes(&self, rsp: &mut Response) {
        rsp.add_attribute("action", "transfer");
        rsp.add_attribute("token_id", self.token_id);
        rsp.add_attribute("amount", self.amount);
        if let Some(from) = self.from {
            rsp.add_attribute("from", from.to_string());
        }
        if let Some(to) = self.to {
            rsp.add_attribute("to", to.to_string());
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
        rsp.add_attribute("action", "set_metadata");
        rsp.add_attribute("url", self.url);
        rsp.add_attribute("token_id", self.token_id);
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
        rsp.add_attribute("action", "approve_all");
        rsp.add_attribute("sender", self.sender.to_string());
        rsp.add_attribute("operator", self.operator.to_string());
        rsp.add_attribute("approved", (self.approved as u32).to_string());
    }
}
