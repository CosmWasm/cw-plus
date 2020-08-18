use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{CanonicalAddr, Env, ReadonlyStorage, Storage};
use cosmwasm_storage::{singleton, singleton_read, ReadonlySingleton, Singleton};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug, Default)]
pub struct AtomicSwap {
    /// This is hex-encoded sha-256 hash of the preimage (must be 32*2 = 64 chars)
    pub hash: String,
    pub recipient: CanonicalAddr,
    pub source: CanonicalAddr,
    pub end_height: u64,
    pub end_time: u64,
}

impl AtomicSwap {
    pub fn is_expired(&self, env: &Env) -> bool {
        (self.end_height != 0 && env.block.height >= self.end_height)
            || (self.end_time != 0 && env.block.time >= self.end_time)
    }
}

pub const ATOMIC_SWAP_KEY: &[u8] = b"atomic_swap";

// config is all config information
pub fn atomic_swap<S: Storage>(storage: &mut S) -> Singleton<S, AtomicSwap> {
    singleton(storage, ATOMIC_SWAP_KEY)
}

pub fn atomic_swap_read<S: ReadonlyStorage>(storage: &S) -> ReadonlySingleton<S, AtomicSwap> {
    singleton_read(storage, ATOMIC_SWAP_KEY)
}
