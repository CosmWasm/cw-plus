use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;

use cosmwasm_std::{CosmosMsg, Decimal, Empty};
use cw0::Expiration;

use crate::msg::Vote;

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Cw3QueryMsg {
    /// Returns the threshold rules that would be used for a new proposal that was
    /// opened right now. The threshold rules do not change often, but the `total_weight`
    /// in the response may easily differ from that used in previously opened proposals.
    /// Returns ThresholdResponse.
    Threshold {},
    /// Returns details of the proposal state. Returns ProposalResponse.
    Proposal { proposal_id: u64 },
    /// Iterate over details of all proposals from oldest to newest. Returns ProposalListResponse
    ListProposals {
        start_after: Option<u64>,
        limit: Option<u32>,
    },
    /// Iterate reverse over details of all proposals, this is useful to easily query
    /// only the most recent proposals (to get updates). Returns ProposalListResponse
    ReverseProposals {
        start_before: Option<u64>,
        limit: Option<u32>,
    },
    /// Query the vote made by the given voter on `proposal_id`. This should
    /// return an error if there is no such proposal. It will return a None value
    /// if the proposal exists but the voter did not vote. Returns VoteResponse
    Vote { proposal_id: u64, voter: String },
    /// Iterate (with pagination) over all votes for this proposal. The ordering is arbitrary,
    /// unlikely to be sorted by address. But ordering is consistent and pagination from the end
    /// of each page will cover all votes for the proposal. Returns VoteListResponse
    ListVotes {
        proposal_id: u64,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// Voter extension: Returns VoterResponse
    Voter { address: String },
    /// ListVoters extension: Returns VoterListResponse
    ListVoters {
        start_after: Option<String>,
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

    /// Declares a percentage of the total weight that must cast Yes votes, in order for
    /// a proposal to pass. The passing weight is computed over the total weight minus the weight of the
    /// abstained votes.
    ///
    /// This is useful for similar circumstances as `AbsoluteCount`, where we have a relatively
    /// small set of voters, and participation is required.
    /// It is understood that if the voting set (group) changes between different proposals that
    /// refer to the same group, each proposal will work with a different set of voter weights
    /// (the ones snapshotted at proposal creation), and the passing weight for each proposal
    /// will be computed based on the absolute percentage, times the total weights of the members
    /// at the time of each proposal creation.
    ///
    /// Example: we set `percentage` to 51%. Proposal 1 starts when there is a `total_weight` of 5.
    /// This will require 3 weight of Yes votes in order to pass. Later, the Proposal 2 starts but the
    /// `total_weight` of the group has increased to 9. That proposal will then automatically
    /// require 5 Yes of 9 to pass, rather than 3 yes of 9 as would be the case with `AbsoluteCount`.
    AbsolutePercentage {
        percentage: Decimal,
        total_weight: u64,
    },

    /// In addition to a `threshold`, declares a `quorum` of the total votes that must participate
    /// in the election in order for the vote to be considered at all. Within the votes that
    /// were cast, it requires `threshold` votes in favor. That is calculated by ignoring
    /// the Abstain votes (they count towards `quorum`, but do not influence `threshold`).
    /// That is, we calculate `Yes / (Yes + No + Veto)` and compare it with `threshold` to consider
    /// if the proposal was passed.
    ///
    /// It is rather difficult for a proposal of this type to pass early. That can only happen if
    /// the required quorum has been already met, and there are already enough Yes votes for the
    /// proposal to pass.
    ///
    /// 30% Yes votes, 10% No votes, and 20% Abstain would pass early if quorum <= 60%
    /// (who has cast votes) and if the threshold is <= 37.5% (the remaining 40% voting
    /// no => 30% yes + 50% no). Once the voting period has passed with no additional votes,
    /// that same proposal would be considered successful if quorum <= 60% and threshold <= 75%
    /// (percent in favor if we ignore abstain votes).
    ///
    /// This type is more common in general elections, where participation is often expected to
    /// be low, and `AbsolutePercentage` would either be too high to pass anything,
    /// or allow low percentages to pass, independently of if there was high participation in the
    /// election or not.
    ThresholdQuorum {
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
    pub status: Status,
    pub expires: Expiration,
    /// This is the threshold that is applied to this proposal. Both the rules of the voting contract,
    /// as well as the total_weight of the voting group may have changed since this time. That means
    /// that the generic `Threshold{}` query does not provide valid information for existing proposals.
    pub threshold: ThresholdResponse,
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

/// Returns the vote (opinion as well as weight counted) as well as
/// the address of the voter who submitted it
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct VoteInfo {
    pub voter: String,
    pub vote: Vote,
    pub weight: u64,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct VoteResponse {
    pub vote: Option<VoteInfo>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct VoterResponse {
    pub weight: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct VoterListResponse {
    pub voters: Vec<VoterDetail>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct VoterDetail {
    pub addr: String,
    pub weight: u64,
}
