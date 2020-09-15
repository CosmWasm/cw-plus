use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{ReadonlyStorage, Storage};
use cosmwasm_storage::{singleton, singleton_read, ReadonlySingleton, Singleton};
use cw0::Expiration;

use crate::msg::Voter;

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug, Default)]
pub struct Voters {
    // TODO: use a canonical address representation?
    pub voters: Vec<Voter>,
    pub required_weight: u64,
    pub total_weight: u64,
    pub max_voting_period: Expiration,
}

pub const VOTERS_KEY: &[u8] = b"voters";

// config is all config information
pub fn voters<S: Storage>(storage: &mut S) -> Singleton<S, Voters> {
    singleton(storage, VOTERS_KEY)
}

pub fn voters_read<S: ReadonlyStorage>(storage: &S) -> ReadonlySingleton<S, Voters> {
    singleton_read(storage, VOTERS_KEY)
}
