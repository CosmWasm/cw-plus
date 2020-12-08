use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::convert::TryInto;

use cosmwasm_std::{BlockInfo, CosmosMsg, Empty, Order, StdError, StdResult, Storage};

use cw0::{Duration, Expiration};
use cw3::{Status, Vote};
use cw4::Cw4Contract;
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, Map, MultiIndex, U64Key};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Config {
    pub required_weight: u64,
    pub max_voting_period: Duration,
    // Total weight and voters are queried from this contract
    pub group_addr: Cw4Contract,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Proposal {
    pub title: String,
    pub description: String,
    pub start_height: u64,
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
    pub fn current_status(&self, block: &BlockInfo) -> Status {
        let mut status = self.status;

        // if open, check if voting is passed or timed out
        if status == Status::Open && self.yes_weight >= self.required_weight {
            status = Status::Passed;
        }
        if status == Status::Open && self.expires.is_expired(block) {
            status = Status::Rejected;
        }

        status
    }
}

pub fn max_proposal_height(storage: &dyn Storage) -> StdResult<Option<u64>> {
    // TODO: this is O(open proposals), with clever composite secondary indexes (status, height), we may get this to O(1)
    let heights: StdResult<Vec<u64>> = proposals()
        .idx
        .status
        .items(
            storage,
            &status_index(Status::Open),
            None,
            None,
            Order::Ascending,
        )
        .map(|res| Ok(res?.1.start_height))
        .collect();
    Ok(heights?.into_iter().max())
}

// we cast a ballot with our chosen vote and a given weight
// stored under the key that voted
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Ballot {
    pub weight: u64,
    pub vote: Vote,
}

// unique items
pub const CONFIG: Item<Config> = Item::new(b"config");
pub const PROPOSAL_COUNT: Item<u64> = Item::new(b"proposal_count");

// multiple-item map
pub const BALLOTS: Map<(U64Key, &[u8]), Ballot> = Map::new(b"votes");

pub fn next_id(store: &mut dyn Storage) -> StdResult<u64> {
    let id: u64 = PROPOSAL_COUNT.may_load(store)?.unwrap_or_default() + 1;
    PROPOSAL_COUNT.save(store, &id)?;
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

// pub const PROPOSALS: Map<U64Key, Proposal> = Map::new(b"proposals");

pub struct ProposalIndexes<'a> {
    pub status: MultiIndex<'a, Proposal>,
}

impl<'a> IndexList<Proposal> for ProposalIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Proposal>> + '_> {
        let v: Vec<&dyn Index<Proposal>> = vec![&self.status];
        Box::new(v.into_iter())
    }
}

/// Returns a value that can be used as a secondary index key in the proposals map
pub fn status_index(status: Status) -> Vec<u8> {
    vec![status as u8]
}

// secondary indexes on state for PROPOSALS to find all open proposals efficiently
pub fn proposals<'a>() -> IndexedMap<'a, U64Key, Proposal, ProposalIndexes<'a>> {
    let indexes = ProposalIndexes {
        status: MultiIndex::new(
            |p| status_index(p.status),
            b"proposals",
            b"proposals__status",
        ),
    };
    IndexedMap::new(b"proposals", indexes)
}
