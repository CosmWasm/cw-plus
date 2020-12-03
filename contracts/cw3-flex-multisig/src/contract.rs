use std::cmp::Ordering;

use cosmwasm_std::{
    attr, to_binary, Binary, BlockInfo, CanonicalAddr, CosmosMsg, Deps, DepsMut, Empty, Env,
    HandleResponse, HumanAddr, InitResponse, MessageInfo, Order, StdResult,
};

use cw0::{maybe_canonical, Expiration};
use cw2::set_contract_version;
use cw3::{
    ProposalListResponse, ProposalResponse, Status, ThresholdResponse, Vote, VoteInfo,
    VoteListResponse, VoteResponse, VoterListResponse, VoterResponse,
};
use cw_storage_plus::Bound;

use crate::error::ContractError;
use crate::msg::{HandleMsg, InitMsg, QueryMsg};
use crate::state::{next_id, parse_id, Ballot, Config, Proposal, BALLOTS, CONFIG, PROPOSALS};
use cw4::Cw4Contract;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cw3-flex-multisig";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn init(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InitMsg,
) -> Result<InitResponse, ContractError> {
    if msg.required_weight == 0 {
        return Err(ContractError::ZeroWeight {});
    }
    // we just convert to canonical to check if this is a valid format
    if deps.api.canonical_address(&msg.group).is_err() {
        return Err(ContractError::InvalidGroup {});
    }

    let group = Cw4Contract(msg.group);
    let total_weight = group.total_weight(&deps.querier)?;

    if total_weight < msg.required_weight {
        return Err(ContractError::UnreachableWeight {});
    }

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let cfg = Config {
        required_weight: msg.required_weight,
        max_voting_period: msg.max_voting_period,
        group,
    };
    CONFIG.save(deps.storage, &cfg)?;

    Ok(InitResponse::default())
}

pub fn handle(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: HandleMsg,
) -> Result<HandleResponse<Empty>, ContractError> {
    match msg {
        HandleMsg::Propose {
            title,
            description,
            msgs,
            latest,
        } => handle_propose(deps, env, info, title, description, msgs, latest),
        HandleMsg::Vote { proposal_id, vote } => handle_vote(deps, env, info, proposal_id, vote),
        HandleMsg::Execute { proposal_id } => handle_execute(deps, env, info, proposal_id),
        HandleMsg::Close { proposal_id } => handle_close(deps, env, info, proposal_id),
    }
}

pub fn handle_propose(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    title: String,
    description: String,
    msgs: Vec<CosmosMsg>,
    // we ignore earliest
    latest: Option<Expiration>,
) -> Result<HandleResponse<Empty>, ContractError> {
    // only members of the multisig can create a proposal
    let raw_sender = deps.api.canonical_address(&info.sender)?;
    let cfg = CONFIG.load(deps.storage)?;

    let vote_power = cfg
        .group
        .is_member(&deps.querier, &raw_sender)?
        .ok_or_else(|| ContractError::Unauthorized {})?;

    // max expires also used as default
    let max_expires = cfg.max_voting_period.after(&env.block);
    let mut expires = latest.unwrap_or(max_expires);
    let comp = expires.partial_cmp(&max_expires);
    if let Some(Ordering::Greater) = comp {
        expires = max_expires;
    } else if comp.is_none() {
        return Err(ContractError::WrongExpiration {});
    }

    let status = if vote_power < cfg.required_weight {
        Status::Open
    } else {
        Status::Passed
    };

    // create a proposal
    let prop = Proposal {
        title,
        description,
        expires,
        msgs,
        status,
        yes_weight: vote_power,
        required_weight: cfg.required_weight,
    };
    let id = next_id(deps.storage)?;
    PROPOSALS.save(deps.storage, id.into(), &prop)?;

    // add the first yes vote from voter
    let ballot = Ballot {
        weight: vote_power,
        vote: Vote::Yes,
    };
    BALLOTS.save(deps.storage, (id.into(), &raw_sender), &ballot)?;

    Ok(HandleResponse {
        messages: vec![],
        attributes: vec![
            attr("action", "propose"),
            attr("sender", info.sender),
            attr("proposal_id", id),
            attr("status", format!("{:?}", prop.status)),
        ],
        data: None,
    })
}

pub fn handle_vote(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    proposal_id: u64,
    vote: Vote,
) -> Result<HandleResponse<Empty>, ContractError> {
    // only members of the multisig can vote
    let raw_sender = deps.api.canonical_address(&info.sender)?;
    let cfg = CONFIG.load(deps.storage)?;

    let vote_power = cfg
        .group
        .is_member(&deps.querier, &raw_sender)?
        .ok_or_else(|| ContractError::Unauthorized {})?;

    // ensure proposal exists and can be voted on
    let mut prop = PROPOSALS.load(deps.storage, proposal_id.into())?;
    if prop.status != Status::Open {
        return Err(ContractError::NotOpen {});
    }
    if prop.expires.is_expired(&env.block) {
        return Err(ContractError::Expired {});
    }

    // cast vote if no vote previously cast
    BALLOTS.update(
        deps.storage,
        (proposal_id.into(), &raw_sender),
        |bal| match bal {
            Some(_) => Err(ContractError::AlreadyVoted {}),
            None => Ok(Ballot {
                weight: vote_power,
                vote,
            }),
        },
    )?;

    // if yes vote, update tally
    if vote == Vote::Yes {
        prop.yes_weight += vote_power;
        // update status when the passing vote comes in
        if prop.yes_weight >= prop.required_weight {
            prop.status = Status::Passed;
        }
        PROPOSALS.save(deps.storage, proposal_id.into(), &prop)?;
    }

    Ok(HandleResponse {
        messages: vec![],
        attributes: vec![
            attr("action", "vote"),
            attr("sender", info.sender),
            attr("proposal_id", proposal_id),
            attr("status", format!("{:?}", prop.status)),
        ],
        data: None,
    })
}

pub fn handle_execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    proposal_id: u64,
) -> Result<HandleResponse, ContractError> {
    // anyone can trigger this if the vote passed

    let mut prop = PROPOSALS.load(deps.storage, proposal_id.into())?;
    // we allow execution even after the proposal "expiration" as long as all vote come in before
    // that point. If it was approved on time, it can be executed any time.
    if prop.status != Status::Passed {
        return Err(ContractError::WrongExecuteStatus {});
    }

    // set it to executed
    prop.status = Status::Executed;
    PROPOSALS.save(deps.storage, proposal_id.into(), &prop)?;

    // dispatch all proposed messages
    Ok(HandleResponse {
        messages: prop.msgs,
        attributes: vec![
            attr("action", "execute"),
            attr("sender", info.sender),
            attr("proposal_id", proposal_id),
        ],
        data: None,
    })
}

pub fn handle_close(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    proposal_id: u64,
) -> Result<HandleResponse<Empty>, ContractError> {
    // anyone can trigger this if the vote passed

    let mut prop = PROPOSALS.load(deps.storage, proposal_id.into())?;
    if [Status::Executed, Status::Rejected, Status::Passed]
        .iter()
        .any(|x| *x == prop.status)
    {
        return Err(ContractError::WrongCloseStatus {});
    }
    if !prop.expires.is_expired(&env.block) {
        return Err(ContractError::NotExpired {});
    }

    // set it to failed
    prop.status = Status::Rejected;
    PROPOSALS.save(deps.storage, proposal_id.into(), &prop)?;

    Ok(HandleResponse {
        messages: vec![],
        attributes: vec![
            attr("action", "close"),
            attr("sender", info.sender),
            attr("proposal_id", proposal_id),
        ],
        data: None,
    })
}

pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Threshold {} => to_binary(&query_threshold(deps)?),
        QueryMsg::Proposal { proposal_id } => to_binary(&query_proposal(deps, env, proposal_id)?),
        QueryMsg::Vote { proposal_id, voter } => to_binary(&query_vote(deps, proposal_id, voter)?),
        QueryMsg::ListProposals { start_after, limit } => {
            to_binary(&list_proposals(deps, env, start_after, limit)?)
        }
        QueryMsg::ReverseProposals {
            start_before,
            limit,
        } => to_binary(&reverse_proposals(deps, env, start_before, limit)?),
        QueryMsg::ListVotes {
            proposal_id,
            start_after,
            limit,
        } => to_binary(&list_votes(deps, proposal_id, start_after, limit)?),
        QueryMsg::Voter { address } => to_binary(&query_voter(deps, address)?),
        QueryMsg::ListVoters { start_after, limit } => {
            to_binary(&list_voters(deps, start_after, limit)?)
        }
    }
}

fn query_threshold(deps: Deps) -> StdResult<ThresholdResponse> {
    let cfg = CONFIG.load(deps.storage)?;
    let total_weight = cfg.group.total_weight(&deps.querier)?;
    Ok(ThresholdResponse::AbsoluteCount {
        weight_needed: cfg.required_weight,
        total_weight,
    })
}

fn query_proposal(deps: Deps, env: Env, id: u64) -> StdResult<ProposalResponse> {
    let prop = PROPOSALS.load(deps.storage, id.into())?;
    let status = prop.current_status(&env.block);
    Ok(ProposalResponse {
        id,
        title: prop.title,
        description: prop.description,
        msgs: prop.msgs,
        expires: prop.expires,
        // TODO: check
        status,
    })
}

// settings for pagination
const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;

fn list_proposals(
    deps: Deps,
    env: Env,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> StdResult<ProposalListResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(Bound::exclusive_int);
    let props: StdResult<Vec<_>> = PROPOSALS
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|p| map_proposal(&env.block, p))
        .collect();

    Ok(ProposalListResponse { proposals: props? })
}

fn reverse_proposals(
    deps: Deps,
    env: Env,
    start_before: Option<u64>,
    limit: Option<u32>,
) -> StdResult<ProposalListResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let end = start_before.map(Bound::exclusive_int);
    let props: StdResult<Vec<_>> = PROPOSALS
        .range(deps.storage, None, end, Order::Descending)
        .take(limit)
        .map(|p| map_proposal(&env.block, p))
        .collect();

    Ok(ProposalListResponse { proposals: props? })
}

fn map_proposal(
    block: &BlockInfo,
    item: StdResult<(Vec<u8>, Proposal)>,
) -> StdResult<ProposalResponse> {
    let (key, prop) = item?;
    let status = prop.current_status(block);
    Ok(ProposalResponse {
        id: parse_id(&key)?,
        title: prop.title,
        description: prop.description,
        msgs: prop.msgs,
        expires: prop.expires,
        status,
    })
}

fn query_vote(deps: Deps, proposal_id: u64, voter: HumanAddr) -> StdResult<VoteResponse> {
    let voter_raw = deps.api.canonical_address(&voter)?;
    let prop = BALLOTS.may_load(deps.storage, (proposal_id.into(), &voter_raw))?;
    let vote = prop.map(|b| b.vote);
    Ok(VoteResponse { vote })
}

fn list_votes(
    deps: Deps,
    proposal_id: u64,
    start_after: Option<HumanAddr>,
    limit: Option<u32>,
) -> StdResult<VoteListResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let canon = maybe_canonical(deps.api, start_after)?;
    let start = canon.map(Bound::exclusive);

    let api = &deps.api;
    let votes: StdResult<Vec<_>> = BALLOTS
        .prefix(proposal_id.into())
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (key, ballot) = item?;
            Ok(VoteInfo {
                voter: api.human_address(&CanonicalAddr::from(key))?,
                vote: ballot.vote,
                weight: ballot.weight,
            })
        })
        .collect();

    Ok(VoteListResponse { votes: votes? })
}

fn query_voter(deps: Deps, voter: HumanAddr) -> StdResult<VoterResponse> {
    let cfg = CONFIG.load(deps.storage)?;
    let voter_raw = deps.api.canonical_address(&voter)?;
    let weight = cfg
        .group
        .is_member(&deps.querier, &voter_raw)?
        .unwrap_or_default();

    Ok(VoterResponse {
        addr: voter,
        weight,
    })
}

fn list_voters(
    deps: Deps,
    start_after: Option<HumanAddr>,
    limit: Option<u32>,
) -> StdResult<VoterListResponse> {
    let cfg = CONFIG.load(deps.storage)?;
    let voters = cfg
        .group
        .list_members(&deps.querier, start_after, limit)?
        .into_iter()
        .map(|member| VoterResponse {
            addr: member.addr,
            weight: member.weight,
        })
        .collect();
    Ok(VoterListResponse { voters })
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coin, from_binary, BankMsg};

    use cw0::Duration;
    use cw2::{get_contract_version, ContractVersion};

    use crate::msg::Voter;

    use super::*;

    fn mock_env_height(height_delta: u64) -> Env {
        let mut env = mock_env();
        env.block.height += height_delta;
        env
    }

    fn mock_env_time(time_delta: u64) -> Env {
        let mut env = mock_env();
        env.block.time += time_delta;
        env
    }

    const OWNER: &str = "admin0001";
    const VOTER1: &str = "voter0001";
    const VOTER2: &str = "voter0002";
    const VOTER3: &str = "voter0003";
    const VOTER4: &str = "voter0004";
    const VOTER5: &str = "voter0005";
    const SOMEBODY: &str = "somebody";

    fn voter<T: Into<HumanAddr>>(addr: T, weight: u64) -> Voter {
        Voter {
            addr: addr.into(),
            weight,
        }
    }

    // this will set up the init for other tests
    fn setup_test_case(
        deps: DepsMut,
        info: MessageInfo,
        required_weight: u64,
        max_voting_period: Duration,
    ) -> Result<InitResponse<Empty>, ContractError> {
        // Init a contract with voters
        let voters = vec![
            voter(&info.sender, 0),
            voter(VOTER1, 1),
            voter(VOTER2, 2),
            voter(VOTER3, 3),
            voter(VOTER4, 4),
            voter(VOTER5, 5),
        ];

        let init_msg = InitMsg {
            voters,
            required_weight,
            max_voting_period,
        };
        init(deps, mock_env(), info, init_msg)
    }

    fn get_tally(deps: Deps, proposal_id: u64) -> u64 {
        // Get all the voters on the proposal
        let voters = QueryMsg::ListVotes {
            proposal_id,
            start_after: None,
            limit: None,
        };
        let votes: VoteListResponse =
            from_binary(&query(deps, mock_env(), voters).unwrap()).unwrap();
        // Sum the weights of the Yes votes to get the tally
        votes
            .votes
            .iter()
            .filter(|&v| v.vote == Vote::Yes)
            .map(|v| v.weight)
            .sum()
    }

    #[test]
    fn test_init_works() {
        let mut deps = mock_dependencies(&[]);
        let info = mock_info(OWNER, &[]);

        let max_voting_period = Duration::Time(1234567);

        // No voters fails
        let init_msg = InitMsg {
            voters: vec![],
            required_weight: 1,
            max_voting_period,
        };
        let res = init(deps.as_mut(), mock_env(), info.clone(), init_msg);

        // Verify
        assert!(res.is_err());
        match res.unwrap_err() {
            ContractError::NoVoters {} => {}
            e => panic!("unexpected error: {}", e),
        }

        // Zero required weight fails
        let init_msg = InitMsg {
            voters: vec![voter(OWNER, 1)],
            required_weight: 0,
            max_voting_period,
        };
        let res = init(deps.as_mut(), mock_env(), info.clone(), init_msg);

        // Verify
        assert!(res.is_err());
        match res.unwrap_err() {
            ContractError::ZeroWeight {} => {}
            e => panic!("unexpected error: {}", e),
        }

        // Total weight less than required weight not allowed
        let required_weight = 100;
        let res = setup_test_case(
            deps.as_mut(),
            info.clone(),
            required_weight,
            max_voting_period,
        );

        // Verify
        assert!(res.is_err());
        match res.unwrap_err() {
            ContractError::UnreachableWeight {} => {}
            e => panic!("unexpected error: {}", e),
        }

        // All valid
        let required_weight = 1;
        setup_test_case(deps.as_mut(), info, required_weight, max_voting_period).unwrap();

        // Verify
        assert_eq!(
            ContractVersion {
                contract: CONTRACT_NAME.to_string(),
                version: CONTRACT_VERSION.to_string(),
            },
            get_contract_version(&deps.storage).unwrap()
        )
    }

    // TODO: query() tests

    #[test]
    fn test_propose_works() {
        let mut deps = mock_dependencies(&[]);

        let required_weight = 4;
        let voting_period = Duration::Time(2000000);

        let info = mock_info(OWNER, &[]);
        setup_test_case(deps.as_mut(), info.clone(), required_weight, voting_period).unwrap();

        let bank_msg = BankMsg::Send {
            from_address: OWNER.into(),
            to_address: SOMEBODY.into(),
            amount: vec![coin(1, "BTC")],
        };
        let msgs = vec![CosmosMsg::Bank(bank_msg)];

        // Only voters can propose
        let info = mock_info(SOMEBODY, &[]);
        let proposal = HandleMsg::Propose {
            title: "Rewarding somebody".to_string(),
            description: "Do we reward her?".to_string(),
            msgs: msgs.clone(),
            latest: None,
        };
        let res = handle(deps.as_mut(), mock_env(), info, proposal.clone());

        // Verify
        assert!(res.is_err());
        match res.unwrap_err() {
            ContractError::Unauthorized {} => {}
            e => panic!("unexpected error: {}", e),
        }

        // Wrong expiration option fails
        let info = mock_info(OWNER, &[]);
        let proposal_wrong_exp = HandleMsg::Propose {
            title: "Rewarding somebody".to_string(),
            description: "Do we reward her?".to_string(),
            msgs: msgs.clone(),
            latest: Some(Expiration::AtHeight(123456)),
        };
        let res = handle(deps.as_mut(), mock_env(), info, proposal_wrong_exp);

        // Verify
        assert!(res.is_err());
        match res.unwrap_err() {
            ContractError::WrongExpiration {} => {}
            e => panic!("unexpected error: {}", e),
        }

        // Proposal from voter works
        let info = mock_info(VOTER3, &[]);
        let res = handle(deps.as_mut(), mock_env(), info, proposal.clone()).unwrap();

        // Verify
        assert_eq!(
            res,
            HandleResponse {
                messages: vec![],
                attributes: vec![
                    attr("action", "propose"),
                    attr("sender", VOTER3),
                    attr("proposal_id", 1),
                    attr("status", "Open"),
                ],
                data: None,
            }
        );

        // Proposal from voter with enough vote power directly passes
        let info = mock_info(VOTER4, &[]);
        let res = handle(deps.as_mut(), mock_env(), info, proposal).unwrap();

        // Verify
        assert_eq!(
            res,
            HandleResponse {
                messages: vec![],
                attributes: vec![
                    attr("action", "propose"),
                    attr("sender", VOTER4),
                    attr("proposal_id", 2),
                    attr("status", "Passed"),
                ],
                data: None,
            }
        );
    }

    #[test]
    fn test_vote_works() {
        let mut deps = mock_dependencies(&[]);

        let required_weight = 3;
        let voting_period = Duration::Time(2000000);

        let info = mock_info(OWNER, &[]);
        setup_test_case(deps.as_mut(), info.clone(), required_weight, voting_period).unwrap();

        // Propose
        let bank_msg = BankMsg::Send {
            from_address: OWNER.into(),
            to_address: SOMEBODY.into(),
            amount: vec![coin(1, "BTC")],
        };
        let msgs = vec![CosmosMsg::Bank(bank_msg)];
        let proposal = HandleMsg::Propose {
            title: "Pay somebody".to_string(),
            description: "Do I pay her?".to_string(),
            msgs,
            latest: None,
        };
        let res = handle(deps.as_mut(), mock_env(), info.clone(), proposal).unwrap();

        // Get the proposal id from the logs
        let proposal_id: u64 = res.attributes[2].value.parse().unwrap();

        // Owner cannot vote (again)
        let yes_vote = HandleMsg::Vote {
            proposal_id,
            vote: Vote::Yes,
        };
        let res = handle(deps.as_mut(), mock_env(), info, yes_vote.clone());

        // Verify
        assert!(res.is_err());
        match res.unwrap_err() {
            ContractError::AlreadyVoted {} => {}
            e => panic!("unexpected error: {}", e),
        }

        // Only voters can vote
        let info = mock_info(SOMEBODY, &[]);
        let res = handle(deps.as_mut(), mock_env(), info, yes_vote.clone());

        // Verify
        assert!(res.is_err());
        match res.unwrap_err() {
            ContractError::Unauthorized {} => {}
            e => panic!("unexpected error: {}", e),
        }

        // But voter1 can
        let info = mock_info(VOTER1, &[]);
        let res = handle(deps.as_mut(), mock_env(), info, yes_vote.clone()).unwrap();

        // Verify
        assert_eq!(
            res,
            HandleResponse {
                messages: vec![],
                attributes: vec![
                    attr("action", "vote"),
                    attr("sender", VOTER1),
                    attr("proposal_id", proposal_id),
                    attr("status", "Open"),
                ],
                data: None,
            }
        );

        // No/Veto votes have no effect on the tally
        // Get the proposal id from the logs
        let proposal_id: u64 = res.attributes[2].value.parse().unwrap();

        // Compute the current tally
        let tally = get_tally(deps.as_ref(), proposal_id);

        // Cast a No vote
        let no_vote = HandleMsg::Vote {
            proposal_id,
            vote: Vote::No,
        };
        let info = mock_info(VOTER2, &[]);
        handle(deps.as_mut(), mock_env(), info, no_vote.clone()).unwrap();

        // Cast a Veto vote
        let veto_vote = HandleMsg::Vote {
            proposal_id,
            vote: Vote::Veto,
        };
        let info = mock_info(VOTER3, &[]);
        handle(deps.as_mut(), mock_env(), info.clone(), veto_vote).unwrap();

        // Verify
        assert_eq!(tally, get_tally(deps.as_ref(), proposal_id));

        // Once voted, votes cannot be changed
        let res = handle(deps.as_mut(), mock_env(), info.clone(), yes_vote.clone());

        // Verify
        assert!(res.is_err());
        match res.unwrap_err() {
            ContractError::AlreadyVoted {} => {}
            e => panic!("unexpected error: {}", e),
        }
        assert_eq!(tally, get_tally(deps.as_ref(), proposal_id));

        // Expired proposals cannot be voted
        let env = match voting_period {
            Duration::Time(duration) => mock_env_time(duration + 1),
            Duration::Height(duration) => mock_env_height(duration + 1),
        };
        let res = handle(deps.as_mut(), env, info, no_vote);

        // Verify
        assert!(res.is_err());
        match res.unwrap_err() {
            ContractError::Expired {} => {}
            e => panic!("unexpected error: {}", e),
        }

        // Vote it again, so it passes
        let info = mock_info(VOTER4, &[]);
        let res = handle(deps.as_mut(), mock_env(), info, yes_vote.clone()).unwrap();

        // Verify
        assert_eq!(
            res,
            HandleResponse {
                messages: vec![],
                attributes: vec![
                    attr("action", "vote"),
                    attr("sender", VOTER4),
                    attr("proposal_id", proposal_id),
                    attr("status", "Passed"),
                ],
                data: None,
            }
        );

        // non-Open proposals cannot be voted
        let info = mock_info(VOTER5, &[]);
        let res = handle(deps.as_mut(), mock_env(), info, yes_vote);

        // Verify
        assert!(res.is_err());
        match res.unwrap_err() {
            ContractError::NotOpen {} => {}
            e => panic!("unexpected error: {}", e),
        }
    }

    #[test]
    fn test_execute_works() {
        let mut deps = mock_dependencies(&[]);

        let required_weight = 3;
        let voting_period = Duration::Time(2000000);

        let info = mock_info(OWNER, &[]);
        setup_test_case(deps.as_mut(), info.clone(), required_weight, voting_period).unwrap();

        // Propose
        let bank_msg = BankMsg::Send {
            from_address: OWNER.into(),
            to_address: SOMEBODY.into(),
            amount: vec![coin(1, "BTC")],
        };
        let msgs = vec![CosmosMsg::Bank(bank_msg)];
        let proposal = HandleMsg::Propose {
            title: "Pay somebody".to_string(),
            description: "Do I pay her?".to_string(),
            msgs: msgs.clone(),
            latest: None,
        };
        let res = handle(deps.as_mut(), mock_env(), info.clone(), proposal).unwrap();

        // Get the proposal id from the logs
        let proposal_id: u64 = res.attributes[2].value.parse().unwrap();

        // Only Passed can be executed
        let execution = HandleMsg::Execute { proposal_id };
        let res = handle(deps.as_mut(), mock_env(), info.clone(), execution.clone());

        // Verify
        assert!(res.is_err());
        match res.unwrap_err() {
            ContractError::WrongExecuteStatus {} => {}
            e => panic!("unexpected error: {}", e),
        }

        // Vote it, so it passes
        let vote = HandleMsg::Vote {
            proposal_id,
            vote: Vote::Yes,
        };
        let info = mock_info(VOTER3, &[]);
        let res = handle(deps.as_mut(), mock_env(), info.clone(), vote).unwrap();

        // Verify
        assert_eq!(
            res,
            HandleResponse {
                messages: vec![],
                attributes: vec![
                    attr("action", "vote"),
                    attr("sender", VOTER3),
                    attr("proposal_id", proposal_id),
                    attr("status", "Passed"),
                ],
                data: None,
            }
        );

        // In passing: Try to close Passed fails
        let closing = HandleMsg::Close { proposal_id };
        let res = handle(deps.as_mut(), mock_env(), info, closing);

        // Verify
        assert!(res.is_err());
        match res.unwrap_err() {
            ContractError::WrongCloseStatus {} => {}
            e => panic!("unexpected error: {}", e),
        }

        // Execute works. Anybody can execute Passed proposals
        let info = mock_info(SOMEBODY, &[]);
        let res = handle(deps.as_mut(), mock_env(), info.clone(), execution).unwrap();

        // Verify
        assert_eq!(
            res,
            HandleResponse {
                messages: msgs,
                attributes: vec![
                    attr("action", "execute"),
                    attr("sender", SOMEBODY),
                    attr("proposal_id", proposal_id),
                ],
                data: None,
            }
        );

        // In passing: Try to close Executed fails
        let closing = HandleMsg::Close { proposal_id };
        let res = handle(deps.as_mut(), mock_env(), info, closing);

        // Verify
        assert!(res.is_err());
        match res.unwrap_err() {
            ContractError::WrongCloseStatus {} => {}
            e => panic!("unexpected error: {}", e),
        }
    }

    #[test]
    fn test_close_works() {
        let mut deps = mock_dependencies(&[]);

        let required_weight = 3;
        let voting_period = Duration::Height(2000000);

        let info = mock_info(OWNER, &[]);
        setup_test_case(deps.as_mut(), info.clone(), required_weight, voting_period).unwrap();

        // Propose
        let bank_msg = BankMsg::Send {
            from_address: OWNER.into(),
            to_address: SOMEBODY.into(),
            amount: vec![coin(1, "BTC")],
        };
        let msgs = vec![CosmosMsg::Bank(bank_msg)];
        let proposal = HandleMsg::Propose {
            title: "Pay somebody".to_string(),
            description: "Do I pay her?".to_string(),
            msgs: msgs.clone(),
            latest: None,
        };
        let res = handle(deps.as_mut(), mock_env(), info.clone(), proposal).unwrap();

        // Get the proposal id from the logs
        let proposal_id: u64 = res.attributes[2].value.parse().unwrap();

        let closing = HandleMsg::Close { proposal_id };

        // Anybody can close
        let info = mock_info(SOMEBODY, &[]);

        // Non-expired proposals cannot be closed
        let res = handle(deps.as_mut(), mock_env(), info.clone(), closing.clone());

        // Verify
        assert!(res.is_err());
        match res.unwrap_err() {
            ContractError::NotExpired {} => {}
            e => panic!("unexpected error: {}", e),
        }

        // Expired proposals can be closed
        let info = mock_info(OWNER, &[]);

        let proposal = HandleMsg::Propose {
            title: "(Try to) pay somebody".to_string(),
            description: "Pay somebody after time?".to_string(),
            msgs: msgs.clone(),
            latest: Some(Expiration::AtHeight(123456)),
        };
        let res = handle(deps.as_mut(), mock_env(), info.clone(), proposal).unwrap();

        // Get the proposal id from the logs
        let proposal_id: u64 = res.attributes[2].value.parse().unwrap();

        let closing = HandleMsg::Close { proposal_id };

        // Close expired works
        let env = mock_env_height(1234567);
        let res = handle(
            deps.as_mut(),
            env,
            mock_info(SOMEBODY, &[]),
            closing.clone(),
        )
        .unwrap();

        // Verify
        assert_eq!(
            res,
            HandleResponse {
                messages: vec![],
                attributes: vec![
                    attr("action", "close"),
                    attr("sender", SOMEBODY),
                    attr("proposal_id", proposal_id),
                ],
                data: None,
            }
        );

        // Trying to close it again fails
        let res = handle(deps.as_mut(), mock_env(), info, closing);

        // Verify
        assert!(res.is_err());
        match res.unwrap_err() {
            ContractError::WrongCloseStatus {} => {}
            e => panic!("unexpected error: {}", e),
        }
    }
}
