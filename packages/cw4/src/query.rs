use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::HumanAddr;

use crate::msg::Member;

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Cw4QueryMsg {
    /// Return AdminResponse
    Admin {},
    // TODO: this also needs raw query access
    /// Return TotalWeightResponse
    TotalWeight {},
    /// Returns MembersListResponse
    ListMembers {
        start_after: Option<HumanAddr>,
        limit: Option<u32>,
    },
    // TODO: this also needs raw query access
    /// Returns MemberResponse
    Member { addr: HumanAddr },
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct MemberListResponse {
    pub members: Vec<Member>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct MemberResponse {
    pub weight: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct AdminResponse {
    pub admin: Option<HumanAddr>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct TotalWeightResponse {
    pub weight: u64,
}

/// TOTAL_KEY is meant for raw queries
pub const TOTAL_KEY: &[u8] = b"total";
pub const MEMBERS_KEY: &[u8] = b"members";

/// member_key is meant for raw queries for one member, given canonical address
pub fn member_key(address: &[u8]) -> Vec<u8> {
    // length encoded members key (update if you change MEMBERS_KEY)
    // inlined here to avoid storage-plus import
    let mut key = b"\x00\x07members".to_vec();
    key.extend_from_slice(address);
    key
}
