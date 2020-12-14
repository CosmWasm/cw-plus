use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::convert::TryInto;

use cosmwasm_std::{BlockInfo, CosmosMsg, Decimal, Empty, StdError, StdResult, Storage, Uint128};

use cw0::{Duration, Expiration};
use cw3::{Status, Vote};
use cw4::Cw4Contract;
use cw_storage_plus::{Item, Map, U64Key};

use crate::msg::Threshold;

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Config {
    pub threshold: Threshold,
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
    /// pass requirements
    pub threshold: Threshold,
    // the total weight when the proposal started (used to calculate percentages)
    pub total_weight: u64,
    // summary of existing votes
    pub votes: Votes,
}

// weight of votes for each option
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Votes {
    pub yes: u64,
    pub no: u64,
    pub abstain: u64,
    pub veto: u64,
}

impl Votes {
    /// sum of all votes
    pub fn total(&self) -> u64 {
        self.yes + self.no + self.abstain + self.veto
    }

    /// create it with a yes vote for this much
    pub fn new(init_weight: u64) -> Self {
        Votes {
            yes: init_weight,
            no: 0,
            abstain: 0,
            veto: 0,
        }
    }

    pub fn add_vote(&mut self, vote: Vote, weight: u64) {
        match vote {
            Vote::Yes => self.yes += weight,
            Vote::Abstain => self.abstain += weight,
            Vote::No => self.no += weight,
            Vote::Veto => self.veto += weight,
        }
    }
}

impl Proposal {
    /// current_status is non-mutable and returns what the status should be.
    /// (designed for queries)
    pub fn current_status(&self, block: &BlockInfo) -> Status {
        let mut status = self.status;

        // if open, check if voting is passed or timed out
        if status == Status::Open && self.is_passed(block) {
            status = Status::Passed;
        }
        if status == Status::Open && self.expires.is_expired(block) {
            status = Status::Rejected;
        }

        status
    }

    /// update_status sets the status of the proposal to current_status.
    /// (designed for handler logic)
    pub fn update_status(&mut self, block: &BlockInfo) {
        self.status = self.current_status(block);
    }

    // returns true iff this proposal is sure to pass (even before expiration if no future
    // sequence of possible votes can cause it to fail)
    pub fn is_passed(&self, block: &BlockInfo) -> bool {
        match self.threshold {
            Threshold::AbsoluteCount { weight_needed } => self.votes.yes >= weight_needed,
            Threshold::AbsolutePercentage { percentage_needed } => {
                self.votes.yes >= votes_needed(self.total_weight, percentage_needed)
            }
            Threshold::ThresholdQuora { threshold, quroum } => {
                let total = self.votes.total();
                // this one is tricky, as we have two compares:
                if self.expires.is_expired(block) {
                    // * if we have closed yet, we need quorum% of total votes to have voted,
                    //   and threshold% of yes votes (from those who voted)
                    total >= votes_needed(self.total_weight, quroum)
                        && self.votes.yes >= votes_needed(total, threshold)
                } else {
                    // * if we have not closed yet, we need threshold% of yes votes (from 100% voters)
                    //   as we are sure this cannot change with any possible sequence of future votes
                    self.votes.yes >= votes_needed(self.total_weight, threshold)
                }
            }
        }
    }
}

// this is a helper function so Decimal works with u64 rather than Uint128
// also, we must *round up* here, as we need 8, not 7 votes to reach 50% of 15 total
fn votes_needed(weight: u64, percentage: Decimal) -> u64 {
    // we multiply by 1million to detect rounding issues
    const FACTOR: u128 = 1_000_000;
    let applied = percentage * Uint128(FACTOR * weight as u128);
    let rounded = (applied.u128() / FACTOR) as u64;
    if applied.u128() % FACTOR > 0 {
        rounded + 1
    } else {
        rounded
    }
}

// we cast a ballot with our chosen vote and a given weight
// stored under the key that voted
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Ballot {
    pub weight: u64,
    pub vote: Vote,
}

// unique items
pub const CONFIG: Item<Config> = Item::new("config");
pub const PROPOSAL_COUNT: Item<u64> = Item::new("proposal_count");

// multiple-item map
pub const BALLOTS: Map<(U64Key, &[u8]), Ballot> = Map::new("votes");
pub const PROPOSALS: Map<U64Key, Proposal> = Map::new("proposals");

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

#[cfg(test)]
mod test {
    use super::*;
    use cosmwasm_std::testing::mock_env;

    #[test]
    fn count_votes() {
        let mut votes = Votes::new(5);
        votes.add_vote(Vote::No, 10);
        votes.add_vote(Vote::Veto, 20);
        votes.add_vote(Vote::Yes, 30);
        votes.add_vote(Vote::Abstain, 40);

        assert_eq!(votes.total(), 105);
        assert_eq!(votes.yes, 35);
        assert_eq!(votes.no, 10);
        assert_eq!(votes.veto, 20);
        assert_eq!(votes.abstain, 40);
    }

    #[test]
    // we ensure this rounds up (as it calculates needed votes)
    fn votes_needed_rounds_properly() {
        // round up right below 1
        assert_eq!(1, votes_needed(3, Decimal::permille(333)));
        // round up right over 1
        assert_eq!(2, votes_needed(3, Decimal::permille(334)));
        assert_eq!(11, votes_needed(30, Decimal::permille(334)));

        // exact matches don't round
        assert_eq!(17, votes_needed(34, Decimal::percent(50)));
        assert_eq!(12, votes_needed(48, Decimal::percent(25)));
    }

    fn check_is_passed(
        threshold: Threshold,
        votes: Votes,
        total_weight: u64,
        is_expired: bool,
    ) -> bool {
        let block = mock_env().block;
        let expires = match is_expired {
            true => Expiration::AtHeight(block.height - 5),
            false => Expiration::AtHeight(block.height + 100),
        };
        let prop = Proposal {
            title: "Demo".to_string(),
            description: "Info".to_string(),
            start_height: 100,
            expires,
            msgs: vec![],
            status: Status::Open,
            threshold,
            total_weight,
            votes,
        };
        prop.is_passed(&block)
    }

    #[test]
    fn proposal_passed_absolute_count() {
        let fixed = Threshold::AbsoluteCount { weight_needed: 10 };
        let mut votes = Votes::new(7);
        votes.add_vote(Vote::Veto, 4);
        // same expired or not, total_weight or whatever
        assert_eq!(
            false,
            check_is_passed(fixed.clone(), votes.clone(), 30, false)
        );
        assert_eq!(
            false,
            check_is_passed(fixed.clone(), votes.clone(), 30, true)
        );
        // a few more yes votes and we are good
        votes.add_vote(Vote::Yes, 3);
        assert_eq!(
            true,
            check_is_passed(fixed.clone(), votes.clone(), 30, false)
        );
        assert_eq!(
            true,
            check_is_passed(fixed.clone(), votes.clone(), 30, true)
        );
    }

    #[test]
    fn proposal_passed_absolute_percentage() {
        let percent = Threshold::AbsolutePercentage {
            percentage_needed: Decimal::percent(50),
        };
        let mut votes = Votes::new(7);
        votes.add_vote(Vote::No, 4);
        votes.add_vote(Vote::Abstain, 2);
        // same expired or not, if total > 2 * yes
        assert_eq!(
            false,
            check_is_passed(percent.clone(), votes.clone(), 15, false)
        );
        assert_eq!(
            false,
            check_is_passed(percent.clone(), votes.clone(), 15, true)
        );

        // if the total were a bit lower, this would pass
        assert_eq!(
            true,
            check_is_passed(percent.clone(), votes.clone(), 14, false)
        );
        assert_eq!(
            true,
            check_is_passed(percent.clone(), votes.clone(), 14, true)
        );
    }
}
