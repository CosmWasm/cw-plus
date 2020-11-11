use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Binary, Storage};
use cosmwasm_storage::{
    prefixed, prefixed_read, singleton, singleton_read, PrefixedStorage, ReadonlyPrefixedStorage,
    ReadonlySingleton, Singleton,
};

pub static CONFIG_KEY: &[u8] = b"config";
const BEACONS_KEY: &[u8] = b"beacons";
const BOUNTIES_KEY: &[u8] = b"bounties";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub pubkey: Binary,
    pub bounty_denom: String,
}

pub fn config<S: Storage>(storage: &mut S) -> Singleton<S, Config> {
    singleton(storage, CONFIG_KEY)
}

pub fn config_read<S: Storage>(storage: &S) -> ReadonlySingleton<S, Config> {
    singleton_read(storage, CONFIG_KEY)
}

pub fn beacons_storage<S: Storage>(storage: &mut S) -> PrefixedStorage<S> {
    prefixed(storage, BEACONS_KEY)
}

pub fn beacons_storage_read<S: Storage>(storage: &S) -> ReadonlyPrefixedStorage<S> {
    prefixed_read(storage, BEACONS_KEY)
}

pub fn bounties_storage<S: Storage>(storage: &mut S) -> PrefixedStorage<S> {
    prefixed(storage, BOUNTIES_KEY)
}

pub fn bounties_storage_read<S: Storage>(storage: &S) -> ReadonlyPrefixedStorage<S> {
    prefixed_read(storage, BOUNTIES_KEY)
}
