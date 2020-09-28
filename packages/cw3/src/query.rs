use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;

use cosmwasm_std::{CosmosMsg, Decimal, Empty, HumanAddr};
use cw0::Expiration;

use crate::msg::Vote;

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Cw3QueryMsg {
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
    /// Voter extension: Returns VoterResponse
    Voter { address: HumanAddr },
    /// Voter extension: Returns VoterListResponse
    ListVoters {
        start_after: Option<HumanAddr>,
        limit: Option<u32>,
    },
}

/// This defines the different ways tallies can happen.
/// It can be extended as needed, but once the spec is frozen,
/// these should not be modified. They are designed to be general.
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ThresholdResponse {
    /// Declares a total weight needed to pass
    /// This usually implies that count_needed is stable, even if total_weight changes
    /// eg. 3 of 5 multisig -> 3 of 6 multisig
    AbsoluteCount {
        weight_needed: u64,
        total_weight: u64,
    },
    /// Declares a percentage of the total weight needed to pass
    /// This implies the percentage is stable, when total_weight changes
    /// eg. at 50.1%, we go from needing 51/100 to needing 101/200
    ///
    /// Note: percentage_needed = 60% is different than threshold = 60%, quora = 100%
    /// as the first will pass with 60% yes votes and 10% no votes, while the second
    /// will require the others to vote anything (no, abstain...) to pass
    AbsolutePercentage {
        percentage_needed: Decimal,
        total_weight: u64,
    },
    /// Declares a threshold (minimum percentage of votes that must approve)
    /// and a quorum (minimum percentage of voter weight that must vote).
    /// This allows eg. 25% of total weight YES to pass, if we have quorum of 40%
    /// and threshold of 51% and most of the people sit out the election.
    /// This is more common in general elections where participation is expected
    /// to be low.
    ThresholdQuora {
        threshold: Decimal,
        quroum: Decimal,
        total_weight: u64,
    },
}

/// Note, if you are storing custom messages in the proposal,
/// the querier needs to know what possible custom message types
/// those are in order to parse the response
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct ProposalResponse<T = Empty>
where
    T: Clone + fmt::Debug + PartialEq + JsonSchema,
{
    pub id: u64,
    pub title: String,
    pub description: String,
    pub msgs: Vec<CosmosMsg<T>>,
    pub expires: Expiration,
    pub status: Status,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    /// proposal was created, but voting has not yet begun for whatever reason
    Pending,
    /// you can vote on this
    Open,
    /// voting is over and it did not pass
    Rejected,
    /// voting is over and it did pass, but has not yet executed
    Passed,
    /// voting is over it passed, and the proposal was executed
    Executed,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct ProposalListResponse {
    pub proposals: Vec<ProposalResponse>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct VoteListResponse {
    pub votes: Vec<VoteInfo>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct VoteInfo {
    pub voter: HumanAddr,
    pub vote: Vote,
    pub weight: u64,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct VoteResponse {
    pub vote: Option<Vote>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct VoterResponse {
    pub addr: HumanAddr,
    pub weight: u64,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct VoterListResponse {
    pub voters: Vec<VoterResponse>,
}
