use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, BlockInfo, CosmosMsg, Decimal, Empty, StdResult, Storage, Uint128};

use cw3::{Status, Vote};
use cw_storage_plus::{Item, Map};
use cw_utils::{Duration, Expiration, Threshold};

// we multiply by this when calculating needed_votes in order to round up properly
// Note: `10u128.pow(9)` fails as "u128::pow` is not yet stable as a const fn"
const PRECISION_FACTOR: u128 = 1_000_000_000;

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Config {
    pub threshold: Threshold,
    pub total_weight: u64,
    pub max_voting_period: Duration,
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
            } => {
                self.votes.yes
                    >= votes_needed(self.total_weight - self.votes.abstain, percentage_needed)
            }
            Threshold::ThresholdQuorum { threshold, quorum } => {
                // we always require the quorum
                if self.votes.total() < votes_needed(self.total_weight, quorum) {
                    return false;
                }
                if self.expires.is_expired(block) {
                    // If expired, we compare Yes votes against the total number of votes (minus abstain).
                    let opinions = self.votes.total() - self.votes.abstain;
                    self.votes.yes >= votes_needed(opinions, threshold)
                } else {
                    // If not expired, we must assume all non-votes will be cast as No.
                    // We compare threshold against the total weight (minus abstain).
                    let possible_opinions = self.total_weight - self.votes.abstain;
                    self.votes.yes >= votes_needed(possible_opinions, threshold)
                }
            }
        }
    }
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
    pub fn yes(init_weight: u64) -> Self {
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

// this is a helper function so Decimal works with u64 rather than Uint128
// also, we must *round up* here, as we need 8, not 7 votes to reach 50% of 15 total
fn votes_needed(weight: u64, percentage: Decimal) -> u64 {
    let applied = percentage * Uint128::new(PRECISION_FACTOR * weight as u128);
    // Divide by PRECISION_FACTOR, rounding up to the nearest integer
    ((applied.u128() + PRECISION_FACTOR - 1) / PRECISION_FACTOR) as u64
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
pub const BALLOTS: Map<(u64, &Addr), Ballot> = Map::new("votes");
pub const PROPOSALS: Map<u64, Proposal> = Map::new("proposals");

// multiple-item maps
pub const VOTERS: Map<&Addr, u64> = Map::new("voters");

pub fn next_id(store: &mut dyn Storage) -> StdResult<u64> {
    let id: u64 = PROPOSAL_COUNT.may_load(store)?.unwrap_or_default() + 1;
    PROPOSAL_COUNT.save(store, &id)?;
    Ok(id)
}

#[cfg(test)]
mod test {
    use super::*;
    use cosmwasm_std::testing::mock_env;

    #[test]
    fn count_votes() {
        let mut votes = Votes::yes(5);
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
        let mut votes = Votes::yes(7);
        votes.add_vote(Vote::Veto, 4);
        // same expired or not, total_weight or whatever
        assert!(!check_is_passed(fixed.clone(), votes.clone(), 30, false));
        assert!(!check_is_passed(fixed.clone(), votes.clone(), 30, true));
        // a few more yes votes and we are good
        votes.add_vote(Vote::Yes, 3);
        assert!(check_is_passed(fixed.clone(), votes.clone(), 30, false));
        assert!(check_is_passed(fixed, votes, 30, true));
    }

    #[test]
    fn proposal_passed_absolute_percentage() {
        let percent = Threshold::AbsolutePercentage {
            percentage: Decimal::percent(50),
        };
        let mut votes = Votes::yes(7);
        votes.add_vote(Vote::No, 4);
        votes.add_vote(Vote::Abstain, 2);
        // same expired or not, if yes >= ceiling(0.5 * (total - abstained))
        // 7 of (15-2) passes
        assert!(check_is_passed(percent.clone(), votes.clone(), 15, false));
        assert!(check_is_passed(percent.clone(), votes.clone(), 15, true));
        // but 7 of (17-2) fails
        assert!(!check_is_passed(percent.clone(), votes.clone(), 17, false));

        // if the total were a bit lower, this would pass
        assert!(check_is_passed(percent.clone(), votes.clone(), 14, false));
        assert!(check_is_passed(percent, votes, 14, true));
    }

    #[test]
    fn proposal_passed_quorum() {
        let quorum = Threshold::ThresholdQuorum {
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
        assert!(check_is_passed(quorum.clone(), passing.clone(), 30, true));
        // under quorum it is not passing (40% of 33 = 13.2 > 13)
        assert!(!check_is_passed(quorum.clone(), passing.clone(), 33, true));
        // over quorum, threshold passes if we ignore abstain
        // 17 total votes w/ abstain => 40% quorum of 40 total
        // 6 yes / (6 yes + 4 no + 2 votes) => 50% threshold
        assert!(check_is_passed(
            quorum.clone(),
            passes_ignoring_abstain.clone(),
            40,
            true
        ));
        // over quorum, but under threshold fails also
        assert!(!check_is_passed(quorum.clone(), failing, 20, true));

        // now, check with open voting period
        // would pass if closed, but fail here, as remaining votes no -> fail
        assert!(!check_is_passed(quorum.clone(), passing.clone(), 30, false));
        assert!(!check_is_passed(
            quorum.clone(),
            passes_ignoring_abstain.clone(),
            40,
            false
        ));
        // if we have threshold * total_weight as yes votes this must pass
        assert!(check_is_passed(quorum.clone(), passing.clone(), 14, false));
        // all votes have been cast, some abstain
        assert!(check_is_passed(
            quorum.clone(),
            passes_ignoring_abstain,
            17,
            false
        ));
        // 3 votes uncast, if they all vote no, we have 7 yes, 7 no+veto, 2 abstain (out of 16)
        assert!(check_is_passed(quorum, passing, 16, false));
    }

    #[test]
    fn quorum_edge_cases() {
        // when we pass absolute threshold (everyone else voting no, we pass), but still don't hit quorum
        let quorum = Threshold::ThresholdQuorum {
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
        assert!(!check_is_passed(
            quorum.clone(),
            missing_voters.clone(),
            15,
            false
        ));
        assert!(!check_is_passed(quorum.clone(), missing_voters, 15, true));

        // 1 less yes, 3 vetos and this passes only when expired
        let wait_til_expired = Votes {
            yes: 8,
            no: 1,
            abstain: 0,
            veto: 3,
        };
        assert!(!check_is_passed(
            quorum.clone(),
            wait_til_expired.clone(),
            15,
            false
        ));
        assert!(check_is_passed(quorum.clone(), wait_til_expired, 15, true));

        // 9 yes and 3 nos passes early
        let passes_early = Votes {
            yes: 9,
            no: 3,
            abstain: 0,
            veto: 0,
        };
        assert!(check_is_passed(
            quorum.clone(),
            passes_early.clone(),
            15,
            false
        ));
        assert!(check_is_passed(quorum, passes_early, 15, true));
    }
}
