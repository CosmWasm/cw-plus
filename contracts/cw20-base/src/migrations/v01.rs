use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Order, StdResult, Storage, Uint128};
use cosmwasm_storage::Bucket;
use cw20::{AllowanceResponse, Expiration};

/// this takes a v0.1.x store and converts it to a v0.2.x format
pub fn migrate_v01_to_v02<S: Storage>(storage: &mut S) -> StdResult<()> {
    // load all the data that needs to change
    let to_migrate: StdResult<Vec<(Vec<u8>, AllowanceResponse)>> = old_allowances(storage)
        .range(None, None, Order::Ascending)
        .filter_map(|item| {
            match item {
                // pass though errors
                Err(e) => Some(Err(e)),
                // filter out if expiration is none
                Ok((
                    _,
                    OldAllowanceResponse {
                        expires: OldExpiration::Never {},
                        ..
                    },
                )) => None,
                // convert the rest
                Ok((k, v)) => Some(Ok((k, v.into()))),
            }
        })
        .collect();

    // overwrite these ones with the new format
    let mut updated = new_allowances(storage);
    for (k, v) in to_migrate?.into_iter() {
        updated.save(&k, &v)?;
    }

    Ok(())
}

/// this read the allowances bucket in the old format
fn old_allowances<'a, S: Storage>(storage: &'a mut S) -> Bucket<'a, S, OldAllowanceResponse> {
    Bucket::new(PREFIX_ALLOWANCE, storage)
}

/// This allows us to write in the new format
fn new_allowances<'a, S: Storage>(storage: &'a mut S) -> Bucket<'a, S, AllowanceResponse> {
    Bucket::new(PREFIX_ALLOWANCE, storage)
}

const PREFIX_ALLOWANCE: &[u8] = b"allowance";

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug, Default)]
pub struct OldAllowanceResponse {
    pub allowance: Uint128,
    pub expires: OldExpiration,
}

/// Convert the OldAllowanceResponse format into the new one
impl Into<AllowanceResponse> for OldAllowanceResponse {
    fn into(self) -> AllowanceResponse {
        AllowanceResponse {
            allowance: self.allowance,
            expires: self.expires.into(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum OldExpiration {
    /// AtHeight will expire when `env.block.height` >= height
    AtHeight { height: u64 },
    /// AtTime will expire when `env.block.time` >= time
    AtTime { time: u64 },
    /// Never will never expire. Used to distinguish None from Some(Expiration::Never)
    Never {},
}

impl Default for OldExpiration {
    fn default() -> Self {
        OldExpiration::Never {}
    }
}

impl Into<Expiration> for OldExpiration {
    fn into(self) -> Expiration {
        match self {
            OldExpiration::AtHeight { height } => Expiration::AtHeight(height),
            OldExpiration::AtTime { time } => Expiration::AtTime(time),
            OldExpiration::Never {} => Expiration::Never {},
        }
    }
}
