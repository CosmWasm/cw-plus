use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{ReadonlyStorage, Storage};
use cosmwasm_storage::{bucket, bucket_read, Bucket, ReadonlyBucket};
use cw0::Expiration;

use crate::balance::Balance;
use std::fmt;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Default, Copy)]
pub struct Permissions {
    pub delegate: bool,
    pub redelegate: bool,
    pub undelegate: bool,
    pub withdraw: bool,
}

impl fmt::Display for Permissions {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f,
               "delegate: {}, redelegate: {}, undelegate: {}, withdraw: {}",
               self.delegate, self.redelegate, self.undelegate, self.withdraw)
    }
}

pub enum PermissionErr {
    Delegate {},
    Redelegate {},
    Undelegate {},
    Withdraw {},
}

impl Into<String> for PermissionErr {
    fn into(self) -> String {
        return String::from(match self {
            PermissionErr::Redelegate {} => "Redelegate is not allowed",
            PermissionErr::Delegate {} => "Delegate is not allowed",
            PermissionErr::Undelegate {} => "Undelegate is not allowed",
            PermissionErr::Withdraw {} => "Withdraw is not allowed",
        });
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Default)]
pub struct Allowance {
    pub balance: Balance,
    pub expires: Expiration,
    pub permissions: Permissions,
}

const PREFIX_ALLOWANCE: &[u8] = b"allowance";

/// returns a bucket with all allowances (query by subkey)
pub fn allowances<S: Storage>(storage: &mut S) -> Bucket<S, Allowance> {
    bucket(PREFIX_ALLOWANCE, storage)
}

/// returns a bucket with all allowances (query by subkey)
/// (read-only version for queries)
pub fn allowances_read<S: ReadonlyStorage>(storage: &S) -> ReadonlyBucket<S, Allowance> {
    bucket_read(PREFIX_ALLOWANCE, storage)
}
