use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{CosmosMsg, Empty};
use cw3::Vote;
use cw_utils::{Duration, Expiration, Threshold};

#[cw_serde]
pub struct InstantiateMsg {
    pub voters: Vec<Voter>,
    pub threshold: Threshold,
    pub max_voting_period: Duration,
}

#[cw_serde]
pub struct Voter {
    pub addr: String,
    pub weight: u64,
}

// TODO: add some T variants? Maybe good enough as fixed Empty for now
#[cw_serde]
pub enum ExecuteMsg {
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
}

// We can also add this as a cw3 extension
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(cw_utils::ThresholdResponse)]
    Threshold {},
    #[returns(cw3::ProposalResponse)]
    Proposal { proposal_id: u64 },
    #[returns(cw3::ProposalListResponse)]
    ListProposals {
        start_after: Option<u64>,
        limit: Option<u32>,
    },
    #[returns(cw3::ProposalListResponse)]
    ReverseProposals {
        start_before: Option<u64>,
        limit: Option<u32>,
    },
    #[returns(cw3::VoteResponse)]
    Vote { proposal_id: u64, voter: String },
    #[returns(cw3::VoteListResponse)]
    ListVotes {
        proposal_id: u64,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    #[returns(cw3::VoterResponse)]
    Voter { address: String },
    #[returns(cw3::VoterListResponse)]
    ListVoters {
        start_after: Option<String>,
        limit: Option<u32>,
    },
}
