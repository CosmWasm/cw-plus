use cosmwasm_std::{
    to_binary, Api, Binary, CosmosMsg, Empty, Env, Extern, HandleResponse, InitResponse, Querier,
    StdError, StdResult, Storage,
};
use cw2::set_contract_version;

use crate::msg::{HandleMsg, InitMsg, QueryMsg};
use crate::state::{
    ballots, config, config_read, next_id, proposal, proposal_read, voter_weight,
    voter_weight_read, Ballot, Config, Proposal,
};
use cw0::Expiration;
use cw3::{ProposalResponse, Status, ThresholdResponse, Vote};

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
    let mut voters = voter_weight(&mut deps.storage);
    for voter in msg.voters.iter() {
        let key = deps.api.canonical_address(&voter.addr)?;
        voters.save(key.as_slice(), &voter.weight)?;
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
        _ => panic!("unimplemented"),
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
    let weight = voter_weight_read(&deps.storage)
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
        yes_weight: weight,
        required_weight: cfg.required_weight,
    };

    // get next id
    let id = next_id(&mut deps.storage)?;

    // save the proposal
    proposal(&mut deps.storage).save(&id.to_be_bytes(), &prop)?;

    // add the first yes vote from voter
    let ballot = Ballot {
        weight,
        vote: Vote::Yes,
    };
    ballots(&mut deps.storage, id).save(raw_sender.as_slice(), &ballot)?;

    // TODO: add some event attributes
    Ok(HandleResponse::default())
}

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::Threshold {} => to_binary(&query_threshold(deps)?),
        QueryMsg::Proposal { proposal_id } => to_binary(&query_proposal(deps, proposal_id)?),
        _ => panic!("unimplemented"),
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
    Ok(ProposalResponse {
        id,
        title: prop.title,
        description: prop.description,
        msgs: prop.msgs,
        expires: prop.expires,
        status: prop.status,
    })
}

#[cfg(test)]
mod tests {
    // TODO
}
