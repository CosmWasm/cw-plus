use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error::ContractError;
use cosmwasm_std::{CosmosMsg, Decimal, Empty, HumanAddr};
use cw0::{Duration, Expiration};
use cw3::Vote;
use cw4::MemberChangedHookMsg;

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct InitMsg {
    // this is the group contract that contains the member list
    pub group_addr: HumanAddr,
    pub threshold: Threshold,
    pub max_voting_period: Duration,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Threshold {
    /// Declares a total weight needed to pass
    /// This usually implies that count_needed is stable, even if total_weight changes
    /// eg. 3 of 5 multisig -> 3 of 6 multisig
    AbsoluteCount { weight_needed: u64 },
    /// Declares a percentage of the total weight needed to pass
    /// This implies the percentage is stable, when total_weight changes
    /// eg. at 50.1%, we go from needing 51/100 to needing 101/200
    ///
    /// Note: percentage_needed = 60% is different than threshold = 60%, quora = 100%
    /// as the first will pass with 60% yes votes and 10% no votes, while the second
    /// will require the others to vote anything (no, abstain...) to pass
    AbsolutePercentage { percentage_needed: Decimal },
    /// Declares a threshold (minimum percentage of votes that must approve)
    /// and a quorum (minimum percentage of voter weight that must vote).
    /// This allows eg. 25% of total weight YES to pass, if we have quorum of 40%
    /// and threshold of 51% and most of the people sit out the election.
    /// This is more common in general elections where participation is expected
    /// to be low.
    ThresholdQuora { threshold: Decimal, quroum: Decimal },
}

impl Threshold {
    /// returns error if this is an unreachable value,
    /// given a total weight of all members in the group
    pub fn validate(&self, total_weight: u64) -> Result<(), ContractError> {
        match self {
            Threshold::AbsoluteCount { weight_needed } => {
                if weight_needed == 0 {
                    Err(ContractError::ZeroThreshold {})
                } else if *weight_needed > total_weight {
                    Err(ContractError::UnreachableThreshold {})
                } else {
                    Ok(())
                }
            }
            Threshold::AbsolutePercentage { percentage_needed } => {
                valid_percentage(percentage_needed)
            }
            Threshold::ThresholdQuora { threshold, quroum } => {
                valid_percentage(threshold)?;
                valid_percentage(quroum)
            }
        };
    }
}

fn valid_percentage(percent: &Decimal) -> Result<(), ContractError> {
    if percent.is_zero() {
        Err(ContractError::ZeroThreshold {})
    } else if *percent > Decimal::one() {
        Err(ContractError::UnreachableThreshold {})
    } else {
        Ok(())
    }
}

// TODO: add some T variants? Maybe good enough as fixed Empty for now
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    Propose {
        title: String,
        description: String,
        msgs: Vec<CosmosMsg<Empty>>,
        // note: we ignore API-spec'd earliest if passed, always opens immediately
        latest: Option<Expiration>,
    },
    Vote {
        proposal_id: u64,
        vote: Vote,
    },
    Execute {
        proposal_id: u64,
    },
    Close {
        proposal_id: u64,
    },
    /// handle update hook messages from the group contract
    MemberChangedHook(MemberChangedHookMsg),
}

// TODO: add a custom query to return the voter list (all potential voters)
// We can also add this as a cw3 extension
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Return ThresholdResponse
    Threshold {},
    /// Returns ProposalResponse
    Proposal { proposal_id: u64 },
    /// Returns ProposalListResponse
    ListProposals {
        start_after: Option<u64>,
        limit: Option<u32>,
    },
    /// Returns ProposalListResponse
    ReverseProposals {
        start_before: Option<u64>,
        limit: Option<u32>,
    },
    /// Returns VoteResponse
    Vote { proposal_id: u64, voter: HumanAddr },
    /// Returns VoteListResponse
    ListVotes {
        proposal_id: u64,
        start_after: Option<HumanAddr>,
        limit: Option<u32>,
    },
    /// Returns VoterInfo
    Voter { address: HumanAddr },
    /// Returns VoterListResponse
    ListVoters {
        start_after: Option<HumanAddr>,
        limit: Option<u32>,
    },
}
