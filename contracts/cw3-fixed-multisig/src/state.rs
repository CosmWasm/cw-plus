use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Api, CanonicalAddr, ReadonlyStorage, StdResult, Storage};
use cosmwasm_storage::{
    bucket, bucket_read, singleton, singleton_read, Bucket, ReadonlyBucket, ReadonlySingleton, Singleton,
};
use cw0::Expiration;

use crate::msg::Voter;

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug, Default)]
pub struct Config {
    pub required_weight: u64,
    pub total_weight: u64,
    pub max_voting_period: Expiration,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct VoterState {
    pub addr: CanonicalAddr,
    pub weight: u64,
}

impl VoterState {
    pub fn human<A: Api>(&self, api: &A) -> StdResult<Voter> {
        Ok(Voter {
            addr: api.human_address(&self.addr)?,
            weight: self.weight,
        })
    }
}

pub const CONFIG_KEY: &[u8] = b"config";
pub const VOTERS_KEY: &[u8] = b"voters";

pub fn config<S: Storage>(storage: &mut S) -> Singleton<S, Config> {
    singleton(storage, CONFIG_KEY)
}

pub fn config_read<S: ReadonlyStorage>(storage: &S) -> ReadonlySingleton<S, Config> {
    singleton_read(storage, CONFIG_KEY)
}

pub fn voters<S: Storage>(storage: &mut S) -> Bucket<S, VoterState> {
    bucket(VOTERS_KEY, storage)
}

pub fn voters_read<S: ReadonlyStorage>(storage: &S) -> ReadonlyBucket<S, VoterState> {
    bucket_read(VOTERS_KEY, storage)
}
