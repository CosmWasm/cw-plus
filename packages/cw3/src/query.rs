use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;

use cosmwasm_std::{CosmosMsg, Empty};
use cw_utils::{Expiration, ThresholdResponse};

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
    pub proposal_id: u64,
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
