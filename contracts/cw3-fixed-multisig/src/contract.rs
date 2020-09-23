use cosmwasm_std::{
    to_binary, Api, Binary, CosmosMsg, Empty, Env, Extern, HandleResponse, HumanAddr, InitResponse,
    Order, Querier, StdError, StdResult, Storage,
};
use cw2::set_contract_version;

use crate::msg::{HandleMsg, InitMsg, QueryMsg};
use crate::state::{
    ballots, ballots_read, config, config_read, next_id, parse_id, proposal, proposal_read, voters,
    voters_read, Ballot, Config, Proposal,
};
use cw0::Expiration;
use cw3::{ProposalListResponse, ProposalResponse, Status, ThresholdResponse, Vote, VoteResponse};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cw3-fixed-multisig";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    _env: Env,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    set_contract_version(&mut deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let total_weight = msg.voters.iter().map(|v| v.weight).sum();
    let cfg = Config {
        required_weight: msg.required_weight,
        total_weight,
        max_voting_period: msg.max_voting_period,
    };
    config(&mut deps.storage).save(&cfg)?;

    // add all voters
    let mut bucket = voters(&mut deps.storage);
    for voter in msg.voters.iter() {
        let key = deps.api.canonical_address(&voter.addr)?;
        bucket.save(key.as_slice(), &voter.weight)?;
    }
    Ok(InitResponse::default())
}

pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> StdResult<HandleResponse<Empty>> {
    match msg {
        HandleMsg::Propose {
            title,
            description,
            msgs,
            earliest,
            latest,
        } => handle_propose(deps, env, title, description, msgs, earliest, latest),
        HandleMsg::Vote { proposal_id, vote } => handle_vote(deps, env, proposal_id, vote),
        HandleMsg::Execute { proposal_id } => handle_execute(deps, env, proposal_id),
        HandleMsg::Close { proposal_id } => handle_close(deps, env, proposal_id),
    }
}

pub fn handle_propose<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    title: String,
    description: String,
    msgs: Vec<CosmosMsg>,
    // we ignore earliest
    _earliest: Option<Expiration>,
    latest: Option<Expiration>,
) -> StdResult<HandleResponse<Empty>> {
    // only members of the multisig can create a proposal
    let raw_sender = deps.api.canonical_address(&env.message.sender)?;
    let vote_power = voters_read(&deps.storage)
        .may_load(raw_sender.as_slice())?
        .ok_or_else(StdError::unauthorized)?;

    // TODO: using max as default here, also enforce max
    let cfg = config_read(&deps.storage).load()?;
    let expires = latest.unwrap_or(cfg.max_voting_period);

    // create a proposal
    let prop = Proposal {
        title,
        description,
        expires,
        msgs,
        status: Status::Open,
        yes_weight: vote_power,
        required_weight: cfg.required_weight,
    };

    // get next id
    let id = next_id(&mut deps.storage)?;

    // save the proposal
    proposal(&mut deps.storage).save(&id.to_be_bytes(), &prop)?;

    // add the first yes vote from voter
    let ballot = Ballot {
        weight: vote_power,
        vote: Vote::Yes,
    };
    ballots(&mut deps.storage, id).save(raw_sender.as_slice(), &ballot)?;

    // TODO: add some event attributes
    Ok(HandleResponse::default())
}

pub fn handle_vote<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    proposal_id: u64,
    vote: Vote,
) -> StdResult<HandleResponse<Empty>> {
    // only members of the multisig can vote
    let raw_sender = deps.api.canonical_address(&env.message.sender)?;
    let vote_power = voters_read(&deps.storage)
        .may_load(raw_sender.as_slice())?
        .ok_or_else(StdError::unauthorized)?;

    // ensure proposal exists and can be voted on
    let mut prop = proposal_read(&deps.storage).load(&proposal_id.to_be_bytes())?;
    if prop.status != Status::Open {
        return Err(StdError::generic_err("Proposal is not open"));
    }
    if prop.expires.is_expired(&env.block) {
        return Err(StdError::generic_err("Proposal voting period has expired"));
    }

    // cast vote if no vote previously cast
    ballots(&mut deps.storage, proposal_id).update(raw_sender.as_slice(), |bal| match bal {
        Some(_) => Err(StdError::generic_err("Already voted on this proposal")),
        None => Ok(Ballot {
            weight: vote_power,
            vote,
        }),
    })?;

    // if yes vote, update tally
    if vote == Vote::Yes {
        prop.yes_weight += vote_power;
        proposal(&mut deps.storage).save(&proposal_id.to_be_bytes(), &prop)?;
    }

    // TODO: add event attributes
    Ok(HandleResponse::default())
}

pub fn handle_execute<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    proposal_id: u64,
) -> StdResult<HandleResponse<Empty>> {
    // anyone can trigger this if the vote passed

    let mut prop = proposal_read(&deps.storage).load(&proposal_id.to_be_bytes())?;
    if prop.status != Status::Open {
        return Err(StdError::generic_err("Proposal is not open for voting"));
    }
    // we enforce this for now, but maybe it doesn't make sense... since no one can *vote*
    // after expiration, if it is expired but there were enough yes votes, it means it passed
    // in the period. Do we enforce execution then as well?
    if prop.expires.is_expired(&env.block) {
        return Err(StdError::generic_err("Proposal voting period has expired"));
    }
    // ensure it passed
    if prop.yes_weight < prop.required_weight {
        return Err(StdError::generic_err(format!(
            "Insufficient yes votes: {} of needed {}",
            prop.yes_weight, prop.required_weight
        )));
    }

    // set it to executed
    prop.status = Status::Executed;
    proposal(&mut deps.storage).save(&proposal_id.to_be_bytes(), &prop)?;

    // dispatch all proposed messages
    // TODO: add event attributes
    Ok(HandleResponse {
        messages: prop.msgs,
        log: vec![],
        data: None,
    })
}

pub fn handle_close<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    proposal_id: u64,
) -> StdResult<HandleResponse<Empty>> {
    // anyone can trigger this if the vote passed

    let mut prop = proposal_read(&deps.storage).load(&proposal_id.to_be_bytes())?;
    if [Status::Executed, Status::Rejected]
        .iter()
        .any(|x| *x == prop.status)
    {
        return Err(StdError::generic_err("Cannot close completed proposals"));
    }
    if !prop.expires.is_expired(&env.block) {
        return Err(StdError::generic_err(
            "Proposal must expire before you can close it",
        ));
    }
    // ensure it did not pass (think about the above... passed and expired should be EITHER closeable or executable)
    if prop.yes_weight >= prop.required_weight {
        return Err(StdError::generic_err("Already passed, try to execute it"));
    }

    // set it to failed
    prop.status = Status::Rejected;
    proposal(&mut deps.storage).save(&proposal_id.to_be_bytes(), &prop)?;

    // TODO: add event attributes
    Ok(HandleResponse::default())
}

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::Threshold {} => to_binary(&query_threshold(deps)?),
        QueryMsg::Proposal { proposal_id } => to_binary(&query_proposal(deps, proposal_id)?),
        QueryMsg::Vote { proposal_id, voter } => to_binary(&query_vote(deps, proposal_id, voter)?),
        QueryMsg::ListProposals { start_after, limit } => {
            to_binary(&list_proposals(deps, start_after, limit)?)
        }
        QueryMsg::ReverseProposals {
            start_before,
            limit,
        } => to_binary(&reverse_proposals(deps, start_before, limit)?),
        QueryMsg::ListVotes { ..
            // proposal_id,
            // start_after,
            // limit,
        } => panic!("unimplemented"),
    }
}

fn query_threshold<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> StdResult<ThresholdResponse> {
    let cfg = config_read(&deps.storage).load()?;
    Ok(ThresholdResponse::AbsoluteCount {
        weight_needed: cfg.required_weight,
        total_weight: cfg.total_weight,
    })
}

fn query_proposal<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    id: u64,
) -> StdResult<ProposalResponse> {
    let prop = proposal_read(&deps.storage).load(&id.to_be_bytes())?;
    let status = prop.current_status();
    Ok(ProposalResponse {
        id,
        title: prop.title,
        description: prop.description,
        msgs: prop.msgs,
        expires: prop.expires,
        status,
    })
}

// settings for pagination
const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;

fn list_proposals<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> StdResult<ProposalListResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(|id| (id + 1).to_be_bytes().to_vec());
    let props: StdResult<Vec<_>> = proposal_read(&deps.storage)
        .range(start.as_deref(), None, Order::Ascending)
        .take(limit)
        .map(map_proposal)
        .collect();

    Ok(ProposalListResponse { proposals: props? })
}

fn reverse_proposals<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    start_before: Option<u64>,
    limit: Option<u32>,
) -> StdResult<ProposalListResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let end = start_before.map(|id| id.to_be_bytes().to_vec());
    let props: StdResult<Vec<_>> = proposal_read(&deps.storage)
        .range(None, end.as_deref(), Order::Descending)
        .take(limit)
        .map(map_proposal)
        .collect();

    Ok(ProposalListResponse { proposals: props? })
}

fn map_proposal(item: StdResult<(Vec<u8>, Proposal)>) -> StdResult<ProposalResponse> {
    let (key, prop) = item?;
    let status = prop.current_status();
    Ok(ProposalResponse {
        id: parse_id(&key)?,
        title: prop.title,
        description: prop.description,
        msgs: prop.msgs,
        expires: prop.expires,
        status,
    })
}

fn query_vote<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    proposal_id: u64,
    voter: HumanAddr,
) -> StdResult<VoteResponse> {
    let voter_raw = deps.api.canonical_address(&voter)?;
    let prop = ballots_read(&deps.storage, proposal_id).may_load(voter_raw.as_slice())?;
    let vote = prop.map(|b| b.vote);
    Ok(VoteResponse { vote })
}

#[cfg(test)]
mod tests {
    // TODO
}
