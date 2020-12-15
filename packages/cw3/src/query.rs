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
    /// Voter extension: Returns VoterInfo
    Voter { address: HumanAddr },
    /// ListVoters extension: Returns VoterListResponse
    ListVoters {
        start_after: Option<HumanAddr>,
        limit: Option<u32>,
    },
}

/// This defines the different ways tallies can happen.
/// Every contract should support a subset of these, ideally all.
///
/// The total_weight used for calculating success as well as the weights of each
/// individual voter used in tallying should be snapshotted at the beginning of
/// the block at which the proposal starts (this is likely the responsibility of a
/// correct cw4 implementation).
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ThresholdResponse {
    /// Declares that a fixed weight of yes votes is needed to pass.
    /// It does not matter how many no votes are cast, or how many do not vote,
    /// as long as `weight` yes votes are cast.
    ///
    /// This is the simplest format and usually suitable for small multisigs of trusted parties,
    /// like 3 of 5. (weight: 3, total_weight: 5)
    ///
    /// A proposal of this type can pass early as soon as the needed weight of yes votes has been cast.
    AbsoluteCount { weight: u64, total_weight: u64 },

    /// Declares a percentage of the total weight that must cast yes votes in order for
    /// a proposal to pass. As with `AbsoluteCount`, it only matters the sum of yes votes.
    ///
    /// This is useful for similar circumstances as `AbsoluteCount`, where we have a relatively
    /// small set of voters and participation is required. The advantage here is that if the
    /// voting set (group) changes between proposals, the number of votes needed is adjusted
    /// accordingly.
    ///
    /// Example: we set `percentage` to 51%. Proposal 1 starts when there is a `total_weight` of 5.
    /// This will require 3 weight of yes votes in order to pass. Later, the Proposal 2 starts but the
    /// `total_weight` of the group has increased to 9. That proposal will then automatically
    /// require 5 yes of 9 to pass, rather than 3 yes of 9 as would be the case with `AbsoluteCount`.
    ///
    /// A proposal of this type can pass early as soon as the needed weight of yes votes has been cast.
    AbsolutePercentage {
        percentage: Decimal,
        total_weight: u64,
    },

    /// Declares a `quorum` of the total votes that must participate in the election in order
    /// for the vote to be considered at all. Within the votes that were cast, it requires `threshold`
    /// in favor. That is calculated by ignoring the abstain votes (they count towards `quorum`
    /// but do not influence `threshold`). That is, we calculate `yes / (yes + no + veto)`
    /// and compare that with `threshold` to consider if the proposal was passed.
    ///
    /// It is rather difficult for a proposal of this type to pass early. That can only happen if
    /// the required quorum has been already met, and in the case if all remaining voters were
    /// to vote no, the threshold would still be met.
    ///
    /// 30% yes votes, 10% no votes, and 20% abstain would pass early if quorum <= 60%
    /// (who has cast votes) and if the threshold is <= 37.5% (the remaining 40% voting
    /// no => 30% yes + 50% no). Once the voting period has passed with no additional votes,
    /// that same proposal would be considered successful if quorum <= 60% and threshold <= 75%
    /// (percent in favor if we ignore abstain votes).
    ///
    /// This type is more common in general elections where participation is expected to often
    /// be low, and `AbsolutePercentage` would either be too restrictive to pass anything,
    /// or allow low percentages to pass if there was high participation in one election.
    ThresholdQuora {
        threshold: Decimal,
        quorum: Decimal,
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
#[repr(u8)]
pub enum Status {
    /// proposal was created, but voting has not yet begun for whatever reason
    Pending = 1,
    /// you can vote on this
    Open = 2,
    /// voting is over and it did not pass
    Rejected = 3,
    /// voting is over and it did pass, but has not yet executed
    Passed = 4,
    /// voting is over it passed, and the proposal was executed
    Executed = 5,
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
pub struct VoterInfo {
    pub weight: Option<u64>,
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
