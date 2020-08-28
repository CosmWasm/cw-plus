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
fn old_allowances<S: Storage>(storage: &mut S) -> Bucket<S, OldAllowanceResponse> {
    Bucket::new(PREFIX_ALLOWANCE, storage)
}

/// This allows us to write in the new format
fn new_allowances<S: Storage>(storage: &mut S) -> Bucket<S, AllowanceResponse> {
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

pub mod testing {
    use super::*;
    use cosmwasm_std::{Api, CanonicalAddr, HumanAddr};
    use cw2::set_contract_version;

    /// This generates test data as if it came from v0.1.0 - only intended for usage in test cases
    /// TODO: make this more robust - how to manage old state?
    /// Maybe we add export and import functions for MockStorage to generate JSON test vectors?
    /// Maybe we embed the entire v0.1 code here to generate state??
    #[allow(dead_code)]
    pub fn generate_v01_test_data<S: Storage, A: Api>(storage: &mut S, api: &A) -> StdResult<()> {
        // TokenInfo:
        // name: Sample Coin
        // symbol: SAMP
        // decimals: 2
        // total_supply: 777777

        // User1: Balance 123456
        //  - Allowance: Spender1, 5000, AtHeight(5000)
        // User2: Balance 654321
        //  - Allowance: Spender1, 15000, AtTime(1598647517)
        //  - Allowance: Spender2, 77777, Never

        set_contract_version(storage, "crates.io:cw20-base", "v0.1.0")?;
        crate::state::token_info(storage).save(&crate::state::TokenInfo {
            name: "Sample Coin".to_string(),
            symbol: "SAMP".to_string(),
            decimals: 2,
            total_supply: Uint128(777777),
            mint: None,
        })?;

        let user1 = api.canonical_address(&HumanAddr::from("user1"))?;
        let user2 = api.canonical_address(&HumanAddr::from("user2"))?;
        crate::state::balances(storage).save(user1.as_slice(), &Uint128(123456))?;
        crate::state::balances(storage).save(user2.as_slice(), &Uint128(654321))?;

        let spender1 = api.canonical_address(&HumanAddr::from("spender1"))?;
        let spender2 = api.canonical_address(&HumanAddr::from("spender2"))?;
        allowances(storage, &user1).save(
            spender1.as_slice(),
            &OldAllowanceResponse {
                allowance: Uint128(5000),
                expires: OldExpiration::AtHeight { height: 5000 },
            },
        )?;
        allowances(storage, &user2).save(
            spender1.as_slice(),
            &OldAllowanceResponse {
                allowance: Uint128(15000),
                expires: OldExpiration::AtTime { time: 1598647517 },
            },
        )?;
        allowances(storage, &user2).save(
            spender2.as_slice(),
            &OldAllowanceResponse {
                allowance: Uint128(77777),
                expires: OldExpiration::Never {},
            },
        )?;

        Ok(())
    }

    /// this read the allowances bucket in the old format
    fn allowances<'a, S: Storage>(
        storage: &'a mut S,
        owner: &CanonicalAddr,
    ) -> Bucket<'a, S, OldAllowanceResponse> {
        Bucket::multilevel(&[PREFIX_ALLOWANCE, owner.as_slice()], storage)
    }
}

#[cfg(test)]
mod test {
    use super::testing::generate_v01_test_data;
    use super::*;
    use cosmwasm_std::testing::mock_dependencies;

    #[test]
    fn sanity_test_migration() {
        let mut deps = mock_dependencies(20, &[]);

        generate_v01_test_data(&mut deps.storage, &deps.api).unwrap();
        migrate_v01_to_v02(&mut deps.storage).unwrap();
    }
}
