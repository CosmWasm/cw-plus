use cosmwasm_std::HumanAddr;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct InitMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    Create(CreateMsg),
    /// Release sends all tokens to the recipient.
    Release {
        id: String,
        /// This is the preimage, must be exactly 32 bytes in hex (64 chars)
        /// to release: sha256(from_hex(preimage)) == from_hex(hash)
        preimage: String,
    },
    /// Refund returns all remaining tokens to the original sender,
    Refund {
        id: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct CreateMsg {
    /// id is a human-readable name for the swap to use later.
    /// 3-20 bytes of utf-8 text
    pub id: String,
    /// This is hex-encoded sha-256 hash of the preimage (must be 32*2 = 64 chars)
    pub hash: String,
    /// If approved, funds go to the recipient
    pub recipient: HumanAddr,
    /// You can set a last time or block height the contract is valid at.
    /// If *either* is non-zero and below current state, the contract is considered expired,
    /// and will be returned to the original funder.
    pub end_height: u64,
    pub end_time: u64,
}
