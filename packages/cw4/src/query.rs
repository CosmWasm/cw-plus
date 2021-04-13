use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Cw4QueryMsg {
    /// Return AdminResponse
    Admin {},
    /// Return TotalWeightResponse
    TotalWeight {},
    /// Returns MembersListResponse
    ListMembers {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// Returns MemberResponse
    Member {
        addr: String,
        at_height: Option<u64>,
    },
    /// Shows all registered hooks. Returns HooksResponse.
    Hooks {},
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct AdminResponse {
    pub admin: Option<String>,
}

/// A group member has a weight associated with them.
/// This may all be equal, or may have meaning in the app that
/// makes use of the group (eg. voting power)
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Member {
    pub addr: String,
    pub weight: u64,
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
pub struct TotalWeightResponse {
    pub weight: u64,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct HooksResponse {
    pub hooks: Vec<String>,
}

/// TOTAL_KEY is meant for raw queries
pub const TOTAL_KEY: &str = "total";
pub const MEMBERS_KEY: &str = "members";
pub const MEMBERS_CHECKPOINTS: &str = "members__checkpoints";
pub const MEMBERS_CHANGELOG: &str = "members__changelog";

/// member_key is meant for raw queries for one member, given address
pub fn member_key(address: &str) -> Vec<u8> {
    // FIXME: Inlined here to avoid storage-plus import
    let mut key = [b"\x00", &[MEMBERS_KEY.len() as u8], MEMBERS_KEY.as_bytes()].concat();
    key.extend_from_slice(address.as_bytes());
    key
}
