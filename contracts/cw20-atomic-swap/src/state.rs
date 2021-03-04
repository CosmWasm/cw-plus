use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Binary, BlockInfo, CanonicalAddr, Order, StdError, StdResult, Storage};
use cosmwasm_storage::{bucket, bucket_read, prefixed_read, Bucket, ReadonlyBucket};
use cw20::{Balance, Expiration};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct AtomicSwap {
    /// This is the sha-256 hash of the preimage
    pub hash: Binary,
    pub recipient: CanonicalAddr,
    pub source: CanonicalAddr,
    pub expires: Expiration,
    /// Balance in native tokens, or cw20 token
    pub balance: Balance,
}

impl AtomicSwap {
    pub fn is_expired(&self, block: &BlockInfo) -> bool {
        self.expires.is_expired(&block)
    }
}

pub const PREFIX_SWAP: &[u8] = b"atomic_swap";

/// Returns a bucket with all swaps (query by id)
pub fn atomic_swaps(storage: &mut dyn Storage) -> Bucket<AtomicSwap> {
    bucket(storage, PREFIX_SWAP)
}

/// Returns a bucket with all swaps (query by id)
/// (read-only version for queries)
pub fn atomic_swaps_read(storage: &dyn Storage) -> ReadonlyBucket<AtomicSwap> {
    bucket_read(storage, PREFIX_SWAP)
}

/// This returns the list of ids for all active swaps
pub fn all_swap_ids(
    storage: &dyn Storage,
    start: Option<Vec<u8>>,
    limit: usize,
) -> StdResult<Vec<String>> {
    prefixed_read(storage, PREFIX_SWAP)
        .range(start.as_deref(), None, Order::Ascending)
        .take(limit)
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
        let ids = all_swap_ids(&storage, None, 10).unwrap();
        assert_eq!(0, ids.len());
    }

    fn dummy_swap() -> AtomicSwap {
        AtomicSwap {
            recipient: CanonicalAddr(Binary(b"recip".to_vec())),
            source: CanonicalAddr(Binary(b"source".to_vec())),
            expires: Default::default(),
            hash: Binary("hash".into()),
            balance: Default::default(),
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

        let ids = all_swap_ids(&storage, None, 10).unwrap();
        assert_eq!(3, ids.len());
        assert_eq!(
            vec!["assign".to_string(), "lazy".to_string(), "zen".to_string()],
            ids
        )
    }
}
