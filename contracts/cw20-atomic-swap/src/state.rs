use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{
    Binary, CanonicalAddr, Coin, Env, Order, ReadonlyStorage, StdError, StdResult, Storage,
};
use cosmwasm_storage::{bucket, bucket_read, prefixed_read, Bucket, ReadonlyBucket};
use cw20::Expiration;

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug, Default)]
pub struct AtomicSwap {
    /// This is the sha-256 hash of the preimage
    pub hash: Binary,
    pub recipient: CanonicalAddr,
    pub source: CanonicalAddr,
    pub expires: Expiration,
    /// Balance in native tokens
    pub balance: Vec<Coin>,
}

impl AtomicSwap {
    pub fn is_expired(&self, env: &Env) -> bool {
        self.expires.is_expired(&env.block)
    }
}

pub const PREFIX_SWAP: &[u8] = b"atomic_swap";

/// Returns a bucket with all swaps (query by id)
pub fn atomic_swaps<S: Storage>(storage: &mut S) -> Bucket<S, AtomicSwap> {
    bucket(PREFIX_SWAP, storage)
}

/// Returns a bucket with all swaps (query by id)
/// (read-only version for queries)
pub fn atomic_swaps_read<S: ReadonlyStorage>(storage: &S) -> ReadonlyBucket<S, AtomicSwap> {
    bucket_read(PREFIX_SWAP, storage)
}

/// This returns the list of ids for all active swaps
pub fn all_swap_ids<S: ReadonlyStorage>(storage: &S) -> StdResult<Vec<String>> {
    prefixed_read(PREFIX_SWAP, storage)
        .range(None, None, Order::Ascending)
        .map(|(k, _)| String::from_utf8(k).map_err(|_| StdError::invalid_utf8("Parsing swap id")))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    use cosmwasm_std::testing::MockStorage;
    use cosmwasm_std::Binary;

    #[test]
    fn test_no_swap_ids() {
        let storage = MockStorage::new();
        let ids = all_swap_ids(&storage).unwrap();
        assert_eq!(0, ids.len());
    }

    fn dummy_swap() -> AtomicSwap {
        AtomicSwap {
            recipient: CanonicalAddr(Binary(b"recip".to_vec())),
            source: CanonicalAddr(Binary(b"source".to_vec())),
            hash: Binary("hash".into()),
            ..AtomicSwap::default()
        }
    }

    #[test]
    fn test_all_swap_ids() {
        let mut storage = MockStorage::new();
        atomic_swaps(&mut storage)
            .save("lazy".as_bytes(), &dummy_swap())
            .unwrap();
        atomic_swaps(&mut storage)
            .save("assign".as_bytes(), &dummy_swap())
            .unwrap();
        atomic_swaps(&mut storage)
            .save("zen".as_bytes(), &dummy_swap())
            .unwrap();

        let ids = all_swap_ids(&storage).unwrap();
        assert_eq!(3, ids.len());
        assert_eq!(
            vec!["assign".to_string(), "lazy".to_string(), "zen".to_string()],
            ids
        )
    }
}
