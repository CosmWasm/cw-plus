use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{CanonicalAddr, Coin, Env, ReadonlyStorage, Storage};
use cosmwasm_storage::{bucket, bucket_read, Bucket, ReadonlyBucket};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug, Default)]
pub struct AtomicSwap {
    /// This is hex-encoded sha-256 hash of the preimage (must be 32*2 = 64 chars)
    pub hash: String,
    pub recipient: CanonicalAddr,
    pub source: CanonicalAddr,
    pub end_height: u64,
    pub end_time: u64,
    /// Balance in native tokens
    pub balance: Vec<Coin>,
}

impl AtomicSwap {
    pub fn is_expired(&self, env: &Env) -> bool {
        (self.end_height != 0 && env.block.height >= self.end_height)
            || (self.end_time != 0 && env.block.time >= self.end_time)
    }
}

pub const PREFIX_SWAP: &[u8] = b"atomic_swap";

/// Returns a bucket with all swaps (query by id)
pub fn atomic_swaps<S: Storage>(storage: &mut S) -> Bucket<S, AtomicSwap> {
    bucket(PREFIX_SWAP, storage)
}

/// returns a bucket with all swaps (query by id)
/// (read-only version for queries)
pub fn atomic_swaps_read<S: ReadonlyStorage>(storage: &S) -> ReadonlyBucket<S, AtomicSwap> {
    bucket_read(PREFIX_SWAP, storage)
}
