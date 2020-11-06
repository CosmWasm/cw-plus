use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{ Storage};
use cosmwasm_storage::{bucket, bucket_read, Bucket, ReadonlyBucket};
use cw0::{Expiration, NativeBalance};

use std::fmt;

// Permissions struct defines users message execution permissions.
// Could have implemented permissions for each cosmos module(StakingPermissions, GovPermissions etc...)
// But that meant a lot of code for each module. Keeping the permissions inside one struct is more
// optimal. Define other modules permissions here.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Default, Copy)]
pub struct Permissions {
    pub delegate: bool,
    pub redelegate: bool,
    pub undelegate: bool,
    pub withdraw: bool,
}

impl fmt::Display for Permissions {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "staking: {{ delegate: {}, redelegate: {}, undelegate: {}, withdraw: {} }}",
            self.delegate, self.redelegate, self.undelegate, self.withdraw
        )
    }
}

const PREFIX_PERMISSIONS: &[u8] = b"permissions";

/// returns a bucket with all permissions (query by subkey)
pub fn permissions(storage: &mut dyn Storage) -> Bucket<Permissions> {
    bucket(storage, PREFIX_PERMISSIONS)
}

/// returns a bucket with all permissionsk (query by subkey)
/// (read-only version for queries)
pub fn permissions_read(storage: &dyn Storage) -> ReadonlyBucket<Permissions> {
    bucket_read(storage, PREFIX_PERMISSIONS)
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Default)]
pub struct Allowance {
    pub balance: NativeBalance,
    pub expires: Expiration,
}

const PREFIX_ALLOWANCE: &[u8] = b"allowance";

/// returns a bucket with all allowances (query by subkey)
pub fn allowances(storage: &mut dyn Storage) -> Bucket<Allowance> {
    bucket(storage, PREFIX_ALLOWANCE)
}

/// returns a bucket with all allowances (query by subkey)
/// (read-only version for queries)
pub fn allowances_read(storage: &dyn Storage) -> ReadonlyBucket<Allowance> {
    bucket_read(storage, PREFIX_ALLOWANCE)
}
