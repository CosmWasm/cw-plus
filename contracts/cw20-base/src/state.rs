use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{CanonicalAddr, ReadonlyStorage, Storage, Uint128};
use cosmwasm_storage::{
    bucket, bucket_read, singleton, singleton_read, Bucket, ReadonlyBucket,
    ReadonlyPrefixedStorage, ReadonlySingleton, Singleton,
};
use cw20::AllowanceResponse;

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub struct TokenInfo {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub total_supply: Uint128,
    pub mint: Option<MinterData>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct MinterData {
    pub minter: CanonicalAddr,
    /// cap is how many more tokens can be issued by the minter
    pub cap: Option<Uint128>,
}

impl TokenInfo {
    pub fn get_cap(&self) -> Option<Uint128> {
        self.mint.as_ref().and_then(|v| v.cap)
    }
}

const TOKEN_INFO_KEY: &[u8] = b"token_info";
const PREFIX_BALANCE: &[u8] = b"balance";
const PREFIX_ALLOWANCE: &[u8] = b"allowance";

// meta is the token definition as well as the total_supply
pub fn token_info<S: Storage>(storage: &mut S) -> Singleton<S, TokenInfo> {
    singleton(storage, TOKEN_INFO_KEY)
}

pub fn token_info_read<S: ReadonlyStorage>(storage: &S) -> ReadonlySingleton<S, TokenInfo> {
    singleton_read(storage, TOKEN_INFO_KEY)
}

/// balances are state of the erc20 tokens
pub fn balances<S: Storage>(storage: &mut S) -> Bucket<S, Uint128> {
    bucket(storage, PREFIX_BALANCE)
}

/// balances are state of the erc20 tokens (read-only version for queries)
pub fn balances_read<S: ReadonlyStorage>(storage: &S) -> ReadonlyBucket<S, Uint128> {
    bucket_read(storage, PREFIX_BALANCE)
}

pub fn balances_prefix_read<S: ReadonlyStorage>(storage: &S) -> ReadonlyPrefixedStorage<S> {
    ReadonlyPrefixedStorage::new(storage, PREFIX_BALANCE)
}

/// returns a bucket with all allowances authorized by this owner (query it by spender)
pub fn allowances<'a, S: Storage>(
    storage: &'a mut S,
    owner: &CanonicalAddr,
) -> Bucket<'a, S, AllowanceResponse> {
    Bucket::multilevel(storage, &[PREFIX_ALLOWANCE, owner.as_slice()])
}

/// returns a bucket with all allowances authorized by this owner (query it by spender)
/// (read-only version for queries)
pub fn allowances_read<'a, S: ReadonlyStorage>(
    storage: &'a S,
    owner: &CanonicalAddr,
) -> ReadonlyBucket<'a, S, AllowanceResponse> {
    ReadonlyBucket::multilevel(storage, &[PREFIX_ALLOWANCE, owner.as_slice()])
}
