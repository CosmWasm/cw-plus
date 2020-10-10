use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::contract::state::H256;
use crate::types::BlockNumber;

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct InitMsg {
    pub name: String,
    pub block: String,
    pub set_id: u64,
    pub authority_set: String,
    pub max_headers_allowed_to_store: u64,
    pub max_headers_allowed_between_justifications: u64,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum HandleMsg {
    UpdateClient {
        block: String,
        authority_set: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum QueryMsg {
    LatestHeight {},
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema)]
pub struct LatestHeightResponse {
    pub best_header_height: BlockNumber,
    pub best_header_hash: H256,
    pub last_finalized_header_hash: H256,
    pub best_header_commitment_root: H256,
    pub current_authority_set: String,
}
