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
            Threshold::AbsoluteCount {
                weight: weight_needed,
            } => self.votes.yes >= weight_needed,
            Threshold::AbsolutePercentage {
                percentage: percentage_needed,
            } => self.votes.yes >= votes_needed(self.total_weight, percentage_needed),
            Threshold::ThresholdQuora { threshold, quorum } => {
                // this one is tricky, as we have two compares:
                if self.expires.is_expired(block) {
                    // * if we have closed yet, we need quorum% of total votes to have voted (counting abstain)
                    //   and threshold% of yes votes from those who voted (ignoring abstain)
                    let total = self.votes.total();
                    let opinions = total - self.votes.abstain;
                    total >= votes_needed(self.total_weight, quorum)
                        && self.votes.yes >= votes_needed(opinions, threshold)
                } else {
                    // * if we have not closed yet, we need threshold% of yes votes (from 100% voters - abstain)
                    //   as we are sure this cannot change with any possible sequence of future votes
                    // * we also need quorum (which may not always be the case above)
                    self.votes.total() >= votes_needed(self.total_weight, quorum)
                        && self.votes.yes
                            >= votes_needed(self.total_weight - self.votes.abstain, threshold)
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
    // Divide by factor, rounding up to the nearest integer
    ((applied.u128() + FACTOR - 1) / FACTOR) as u64
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
        let fixed = Threshold::AbsoluteCount { weight: 10 };
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
            percentage: Decimal::percent(50),
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

    #[test]
    fn proposal_passed_quorum() {
        let quorum = Threshold::ThresholdQuora {
            threshold: Decimal::percent(50),
            quorum: Decimal::percent(40),
        };
        // all non-yes votes are counted for quorum
        let passing = Votes {
            yes: 7,
            no: 3,
            abstain: 2,
            veto: 1,
        };
        // abstain votes are not counted for threshold => yes / (yes + no + veto)
        let passes_ignoring_abstain = Votes {
            yes: 6,
            no: 4,
            abstain: 5,
            veto: 2,
        };
        // fails any way you look at it
        let failing = Votes {
            yes: 6,
            no: 5,
            abstain: 2,
            veto: 2,
        };

        // first, expired (voting period over)
        // over quorum (40% of 30 = 12), over threshold (7/11 > 50%)
        assert_eq!(
            true,
            check_is_passed(quorum.clone(), passing.clone(), 30, true)
        );
        // under quorum it is not passing (40% of 33 = 13.2 > 13)
        assert_eq!(
            false,
            check_is_passed(quorum.clone(), passing.clone(), 33, true)
        );
        // over quorum, threshold passes if we ignore abstain
        // 17 total votes w/ abstain => 40% quorum of 40 total
        // 6 yes / (6 yes + 4 no + 2 votes) => 50% threshold
        assert_eq!(
            true,
            check_is_passed(quorum.clone(), passes_ignoring_abstain.clone(), 40, true)
        );
        // over quorum, but under threshold fails also
        assert_eq!(
            false,
            check_is_passed(quorum.clone(), failing.clone(), 20, true)
        );

        // now, check with open voting period
        // would pass if closed, but fail here, as remaining votes no -> fail
        assert_eq!(
            false,
            check_is_passed(quorum.clone(), passing.clone(), 30, false)
        );
        assert_eq!(
            false,
            check_is_passed(quorum.clone(), passes_ignoring_abstain.clone(), 40, false)
        );
        // if we have threshold * total_weight as yes votes this must pass
        assert_eq!(
            true,
            check_is_passed(quorum.clone(), passing.clone(), 14, false)
        );
        // all votes have been cast, some abstain
        assert_eq!(
            true,
            check_is_passed(quorum.clone(), passes_ignoring_abstain.clone(), 17, false)
        );
        // 3 votes uncast, if they all vote no, we have 7 yes, 7 no+veto, 2 abstain (out of 16)
        assert_eq!(
            true,
            check_is_passed(quorum.clone(), passing.clone(), 16, false)
        );
    }

    #[test]
    fn quorum_edge_cases() {
        // when we pass absolute threshold (everyone else voting no, we pass), but still don't hit quorum
        let quorum = Threshold::ThresholdQuora {
            threshold: Decimal::percent(60),
            quorum: Decimal::percent(80),
        };

        // try 9 yes, 1 no (out of 15) -> 90% voter threshold, 60% absolute threshold, still no quorum
        // doesn't matter if expired or not
        let missing_voters = Votes {
            yes: 9,
            no: 1,
            abstain: 0,
            veto: 0,
        };
        assert_eq!(
            false,
            check_is_passed(quorum.clone(), missing_voters.clone(), 15, false)
        );
        assert_eq!(
            false,
            check_is_passed(quorum.clone(), missing_voters.clone(), 15, true)
        );

        // 1 less yes, 3 vetos and this passes only when expired
        let wait_til_expired = Votes {
            yes: 8,
            no: 1,
            abstain: 0,
            veto: 3,
        };
        assert_eq!(
            false,
            check_is_passed(quorum.clone(), wait_til_expired.clone(), 15, false)
        );
        assert_eq!(
            true,
            check_is_passed(quorum.clone(), wait_til_expired.clone(), 15, true)
        );

        // 9 yes and 3 nos passes early
        let passes_early = Votes {
            yes: 9,
            no: 3,
            abstain: 0,
            veto: 0,
        };
        assert_eq!(
            true,
            check_is_passed(quorum.clone(), passes_early.clone(), 15, false)
        );
        assert_eq!(
            true,
            check_is_passed(quorum.clone(), passes_early.clone(), 15, true)
        );
    }
}
