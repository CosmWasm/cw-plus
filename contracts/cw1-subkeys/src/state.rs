use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{ReadonlyStorage, Storage};
use cosmwasm_storage::{bucket, bucket_read, Bucket, ReadonlyBucket};
use cw20::Expiration;

use crate::balance::Balance;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Default)]
pub struct Allowance {
    balance: Balance,
    expires: Expiration,
}

const PREFIX_ALLOWANCE: &[u8] = b"allowance";

/// returns a bucket with all allowances (query by subkey)
pub fn allowances<'a, S: Storage>(storage: &'a mut S) -> Bucket<'a, S, Allowance> {
    bucket(PREFIX_ALLOWANCE, storage)
}

/// returns a bucket with all allowances (query by subkey)
/// (read-only version for queries)
pub fn allowances_read<'a, S: ReadonlyStorage>(storage: &'a S) -> ReadonlyBucket<'a, S, Allowance> {
    bucket_read(PREFIX_ALLOWANCE, storage)
}
