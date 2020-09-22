use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::convert::TryInto;

use cosmwasm_std::{
    Api, CanonicalAddr, CosmosMsg, Empty, ReadonlyStorage, StdError, StdResult, Storage,
};
use cosmwasm_storage::{
    bucket, bucket_read, singleton, singleton_read, Bucket, ReadonlyBucket, ReadonlySingleton,
    Singleton,
};
use cw0::Expiration;
use cw3::Status;

use crate::msg::Voter;

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug, Default)]
pub struct Config {
    pub required_weight: u64,
    pub total_weight: u64,
    // TODO: use duration not expiration!
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

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Proposal {
    pub title: String,
    pub description: String,
    pub expires: Expiration,
    pub msgs: Vec<CosmosMsg<Empty>>,
    pub status: Status,
}

pub const CONFIG_KEY: &[u8] = b"config";
pub const PROPOSAL_KEY: &[u8] = b"proposal";
pub const PROPOSAL_COUNTER: &[u8] = b"proposal_count";
pub const VOTERS_KEY: &[u8] = b"voter";

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

pub fn proposal<S: Storage>(storage: &mut S) -> Bucket<S, Proposal> {
    bucket(PROPOSAL_KEY, storage)
}

pub fn proposal_read<S: ReadonlyStorage>(storage: &S) -> ReadonlyBucket<S, Proposal> {
    bucket_read(PROPOSAL_KEY, storage)
}

pub fn next_id<S: Storage>(storage: &mut S) -> StdResult<u64> {
    let mut s = singleton(storage, PROPOSAL_COUNTER);
    let id: u64 = s.may_load()?.unwrap_or_default() + 1;
    s.save(&id)?;
    Ok(id)
}

pub fn parse_id(data: &[u8]) -> StdResult<u64> {
    match data[0..8].try_into() {
        Ok(bytes) => Ok(u64::from_be_bytes(bytes)),
        Err(_) => Err(StdError::generic_err(
            "Corrupted data found. 8 byte expected.",
        )),
    }
}
