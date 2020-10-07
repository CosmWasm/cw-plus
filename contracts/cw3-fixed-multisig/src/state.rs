use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::convert::TryInto;

use cosmwasm_std::{CosmosMsg, Empty, ReadonlyStorage, StdError, StdResult, Storage};
use cosmwasm_storage::{
    bucket, bucket_read, singleton, singleton_read, Bucket, ReadonlyBucket, ReadonlySingleton,
    Singleton,
};
use cw0::{Duration, Expiration};
use cw3::{Status, Vote};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Config {
    pub required_weight: u64,
    pub total_weight: u64,
    pub max_voting_period: Duration,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Proposal {
    pub title: String,
    pub description: String,
    pub expires: Expiration,
    pub msgs: Vec<CosmosMsg<Empty>>,
    pub status: Status,
    /// how many votes have already said yes
    pub yes_weight: u64,
    /// how many votes needed to pass
    pub required_weight: u64,
}

impl Proposal {
    /// TODO: we should get the current BlockInfo and then we can determine this a bit better
    pub fn current_status(&self) -> Status {
        let mut status = self.status;

        // if open, check if voting is passed on timed out
        if status == Status::Open && self.yes_weight >= self.required_weight {
            status = Status::Passed
        }

        status
    }
}

// we cast a ballot with our chosen vote and a given weight
// stored under the key that voted
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Ballot {
    pub weight: u64,
    pub vote: Vote,
}

pub const CONFIG_KEY: &[u8] = b"config";
pub const PROPOSAL_COUNTER: &[u8] = b"proposal_count";

pub const PREFIX_PROPOSAL: &[u8] = b"proposals";
pub const PREFIX_VOTERS: &[u8] = b"voters";
pub const PREFIX_VOTES: &[u8] = b"votes";

pub fn config<S: Storage>(storage: &mut S) -> Singleton<S, Config> {
    singleton(storage, CONFIG_KEY)
}

pub fn config_read<S: ReadonlyStorage>(storage: &S) -> ReadonlySingleton<S, Config> {
    singleton_read(storage, CONFIG_KEY)
}

pub fn voters<S: Storage>(storage: &mut S) -> Bucket<S, u64> {
    bucket(storage, PREFIX_VOTERS)
}

pub fn voters_read<S: ReadonlyStorage>(storage: &S) -> ReadonlyBucket<S, u64> {
    bucket_read(storage, PREFIX_VOTERS)
}

pub fn proposal<S: Storage>(storage: &mut S) -> Bucket<S, Proposal> {
    bucket(storage, PREFIX_PROPOSAL)
}

pub fn proposal_read<S: ReadonlyStorage>(storage: &S) -> ReadonlyBucket<S, Proposal> {
    bucket_read(storage, PREFIX_PROPOSAL)
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

pub fn ballots<S: Storage>(storage: &mut S, proposal_id: u64) -> Bucket<S, Ballot> {
    Bucket::multilevel(storage, &[PREFIX_VOTES, &proposal_id.to_be_bytes()])
}

pub fn ballots_read<S: ReadonlyStorage>(
    storage: &S,
    proposal_id: u64,
) -> ReadonlyBucket<S, Ballot> {
    ReadonlyBucket::multilevel(storage, &[PREFIX_VOTES, &proposal_id.to_be_bytes()])
}
