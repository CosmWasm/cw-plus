use cosmwasm_std::HumanAddr;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct InitMsg {
    // This is hex-encoded sha-256 hash of the preimage (must be 32*2 = 64 chars)
    pub hash: String,
    pub recipient: HumanAddr,
    // you can set a last time or block height the contract is valid at
    // if *either* is non-zero and below current state, the contract is considered expired
    // and will be returned to the original funder
    pub end_height: u64,
    pub end_time: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    /// Release sends all tokens to the recipient.
    Release {
        /// This is the preimage, must be exactly 32 bytes in hex (64 chars)
        /// to release: sha256(from_hex(preimage)) == from_hex(hash)
        preimage: String,
    },
    /// Refund returns all remaining tokens to the original sender,
    Refund {},
}
