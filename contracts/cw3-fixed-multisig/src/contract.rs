use std::cmp::Ordering;

use cosmwasm_std::{
    log, to_binary, Api, Binary, CanonicalAddr, CosmosMsg, Empty, Env, Extern, HandleResponse,
    HumanAddr, InitResponse, Order, Querier, StdError, StdResult, Storage,
};

use cw0::Expiration;
use cw2::set_contract_version;
use cw3::{
    ProposalListResponse, ProposalResponse, Status, ThresholdResponse, Vote, VoteInfo,
    VoteListResponse, VoteResponse, VoterListResponse, VoterResponse,
};

use crate::msg::{HandleMsg, InitMsg, QueryMsg};
use crate::state::{
    ballots, ballots_read, config, config_read, next_id, parse_id, proposal, proposal_read, voters,
    voters_read, Ballot, Config, Proposal,
};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cw3-fixed-multisig";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    if msg.required_weight == 0 {
        return Err(StdError::generic_err("Required weight cannot be zero"));
    }
    if msg.voters.is_empty() {
        return Err(StdError::generic_err("No voters"));
    }
    let weights: StdResult<Vec<u64>> = msg
        .voters
        .iter()
        .map(|v| {
            if v.weight == 0 && v.addr != env.message.sender {
                Err(StdError::generic_err(
                    "Voting weights (except sender's) cannot be zero",
                ))
            } else {
                Ok(v.weight)
            }
        })
        .collect();
    let total_weight = weights?.iter().sum();

    if total_weight < msg.required_weight {
        return Err(StdError::generic_err(
            "Not possible to reach required (passing) weight",
        ));
    }

    set_contract_version(&mut deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

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
            latest,
        } => handle_propose(deps, env, title, description, msgs, latest),
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
    latest: Option<Expiration>,
) -> StdResult<HandleResponse<Empty>> {
    // only members of the multisig can create a proposal
    let raw_sender = deps.api.canonical_address(&env.message.sender)?;
    let vote_power = voters_read(&deps.storage)
        .may_load(raw_sender.as_slice())?
        .ok_or_else(StdError::unauthorized)?;

    let cfg = config_read(&deps.storage).load()?;
    // max expires also used as default
    let max_expires = cfg.max_voting_period.after(&env.block);
    let mut expires = latest.unwrap_or(max_expires);
    if expires.partial_cmp(&max_expires) != Some(Ordering::Less) {
        expires = max_expires;
    }

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
    let id = next_id(&mut deps.storage)?;
    proposal(&mut deps.storage).save(&id.to_be_bytes(), &prop)?;

    // add the first yes vote from voter
    let ballot = Ballot {
        weight: vote_power,
        vote: Vote::Yes,
    };
    ballots(&mut deps.storage, id).save(raw_sender.as_slice(), &ballot)?;

    Ok(HandleResponse {
        messages: vec![],
        log: vec![
            log("action", "propose"),
            log("sender", env.message.sender),
            log("proposal_id", id),
        ],
        data: None,
    })
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
        // update status when the passing vote comes in
        if prop.yes_weight >= prop.required_weight {
            prop.status = Status::Passed;
        }
        proposal(&mut deps.storage).save(&proposal_id.to_be_bytes(), &prop)?;
    }

    Ok(HandleResponse {
        messages: vec![],
        log: vec![
            log("action", "vote"),
            log("sender", env.message.sender),
            log("proposal_id", proposal_id),
            log("status", format!("{:?}", prop.status)),
        ],
        data: None,
    })
}

pub fn handle_execute<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    proposal_id: u64,
) -> StdResult<HandleResponse<Empty>> {
    // anyone can trigger this if the vote passed

    let mut prop = proposal_read(&deps.storage).load(&proposal_id.to_be_bytes())?;
    // we allow execution even after the proposal "expiration" as long as all vote come in before
    // that point. If it was approved on time, it can be executed any time.
    if prop.status != Status::Passed {
        return Err(StdError::generic_err(
            "Proposal must have passed and not yet been executed",
        ));
    }

    // set it to executed
    prop.status = Status::Executed;
    proposal(&mut deps.storage).save(&proposal_id.to_be_bytes(), &prop)?;

    // dispatch all proposed messages
    Ok(HandleResponse {
        messages: prop.msgs,
        log: vec![
            log("action", "execute"),
            log("sender", env.message.sender),
            log("proposal_id", proposal_id),
        ],
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
    if [Status::Executed, Status::Rejected, Status::Passed]
        .iter()
        .any(|x| *x == prop.status)
    {
        return Err(StdError::generic_err(
            "Cannot close completed or passed proposals",
        ));
    }
    if !prop.expires.is_expired(&env.block) {
        return Err(StdError::generic_err(
            "Proposal must expire before you can close it",
        ));
    }

    // set it to failed
    prop.status = Status::Rejected;
    proposal(&mut deps.storage).save(&proposal_id.to_be_bytes(), &prop)?;

    Ok(HandleResponse {
        messages: vec![],
        log: vec![
            log("action", "close"),
            log("sender", env.message.sender),
            log("proposal_id", proposal_id),
        ],
        data: None,
    })
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

fn list_votes<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    proposal_id: u64,
    start_after: Option<HumanAddr>,
    limit: Option<u32>,
) -> StdResult<VoteListResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = calc_range_start(start_after);
    let api = &deps.api;

    let votes: StdResult<Vec<_>> = ballots_read(&deps.storage, proposal_id)
        .range(start.as_deref(), None, Order::Ascending)
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

fn query_voter<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    voter: HumanAddr,
) -> StdResult<VoterResponse> {
    let voter_raw = deps.api.canonical_address(&voter)?;
    let weight = voters_read(&deps.storage)
        .may_load(voter_raw.as_slice())?
        .unwrap_or_default();
    Ok(VoterResponse {
        addr: voter,
        weight,
    })
}

fn list_voters<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    start_after: Option<HumanAddr>,
    limit: Option<u32>,
) -> StdResult<VoterListResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = calc_range_start(start_after);
    let api = &deps.api;

    let voters: StdResult<Vec<_>> = voters_read(&deps.storage)
        .range(start.as_deref(), None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (key, weight) = item?;
            Ok(VoterResponse {
                addr: api.human_address(&CanonicalAddr::from(key))?,
                weight,
            })
        })
        .collect();

    Ok(VoterListResponse { voters: voters? })
}

// this will set the first key after the provided key, by appending a 1 byte
fn calc_range_start(start_after: Option<HumanAddr>) -> Option<Vec<u8>> {
    start_after.map(|human| {
        let mut v = Vec::from(human.0);
        v.push(1);
        v
    })
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::{mock_dependencies, mock_env};
    use cosmwasm_std::{coin, BankMsg};

    use cw0::Duration;
    use cw2::{get_contract_version, ContractVersion};

    use crate::msg::Voter;

    use super::*;

    const OWNER: &str = "admin0001";
    const VOTER1: &str = "voter0001";
    const VOTER2: &str = "voter0002";
    const VOTER3: &str = "voter0003";
    const SOMEBODY: &str = "somebody";

    fn voter<T: Into<HumanAddr>>(addr: T, weight: u64) -> Voter {
        Voter {
            addr: addr.into(),
            weight,
        }
    }

    // this will set up the init for other tests
    fn setup_test_case<S: Storage, A: Api, Q: Querier>(
        mut deps: &mut Extern<S, A, Q>,
        env: Env,
        required_weight: u64,
        max_voting_period: Duration,
    ) -> StdResult<InitResponse<Empty>> {
        // Init a contract with voters
        let voters = vec![
            voter(&env.message.sender, 0),
            voter(VOTER1, 1),
            voter(VOTER2, 2),
            voter(VOTER3, 3),
        ];

        let init_msg = InitMsg {
            voters,
            required_weight,
            max_voting_period,
        };
        init(&mut deps, env, init_msg)
    }

    #[test]
    fn test_init_works() {
        let mut deps = mock_dependencies(20, &[]);
        let env = mock_env(OWNER, &[]);

        let max_voting_period = Duration::Time(1234567);

        // No voters fails
        let init_msg = InitMsg {
            voters: vec![],
            required_weight: 1,
            max_voting_period,
        };
        let res = init(&mut deps, env.clone(), init_msg);

        // Verify
        assert!(res.is_err());
        match res.unwrap_err() {
            StdError::GenericErr { msg, .. } => assert_eq!(&msg, "No voters"),
            e => panic!("unexpected error: {}", e),
        }

        // Zero required weight fails
        let init_msg = InitMsg {
            voters: vec![voter(OWNER, 1)],
            required_weight: 0,
            max_voting_period,
        };
        let res = init(&mut deps, env.clone(), init_msg);

        // Verify
        assert!(res.is_err());
        match res.unwrap_err() {
            StdError::GenericErr { msg, .. } => assert_eq!(&msg, "Required weight cannot be zero"),
            e => panic!("unexpected error: {}", e),
        }

        // Zero weights for voters other than sender not allowed
        let init_msg = InitMsg {
            voters: vec![voter(OWNER, 1), voter(VOTER1, 0)],
            required_weight: 1,
            max_voting_period,
        };
        let res = init(&mut deps, env.clone(), init_msg);

        // Verify
        assert!(res.is_err());
        match res.unwrap_err() {
            StdError::GenericErr { msg, .. } => {
                assert_eq!(&msg, "Voting weights (except sender's) cannot be zero")
            }
            e => panic!("unexpected error: {}", e),
        }

        // Total weight less than required weight not allowed
        let required_weight = 10;
        let res = setup_test_case(&mut deps, env.clone(), required_weight, max_voting_period);

        // Verify
        assert!(res.is_err());
        match res.unwrap_err() {
            StdError::GenericErr { msg, .. } => {
                assert_eq!(&msg, "Not possible to reach required (passing) weight")
            }
            e => panic!("unexpected error: {}", e),
        }

        // All valid
        let required_weight = 1;
        setup_test_case(&mut deps, env, required_weight, max_voting_period).unwrap();

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
        let mut deps = mock_dependencies(20, &[]);

        let required_weight = 3;
        let voting_period = Duration::Time(2000000);

        let env = mock_env(OWNER, &[]);
        setup_test_case(&mut deps, env.clone(), required_weight, voting_period).unwrap();

        let bank_msg = BankMsg::Send {
            from_address: OWNER.into(),
            to_address: SOMEBODY.into(),
            amount: vec![coin(1, "BTC")],
        };
        let msgs = vec![CosmosMsg::Bank(bank_msg)];

        // Only voters can propose
        let env = mock_env(SOMEBODY, &[]);
        let proposal = HandleMsg::Propose {
            title: "Rewarding somebody".to_string(),
            description: "Do we reward her?".to_string(),
            msgs,
            latest: None,
        };
        let res = handle(&mut deps, env, proposal.clone());

        // Verify
        assert!(res.is_err());
        match res.unwrap_err() {
            StdError::Unauthorized { .. } => {}
            e => panic!("unexpected error: {}", e),
        }

        // Proposal from voter works
        let env = mock_env(VOTER3, &[]);
        let res = handle(&mut deps, env, proposal).unwrap();

        // Verify
        assert_eq!(
            res,
            HandleResponse {
                messages: vec![],
                log: vec![
                    log("action", "propose"),
                    log("sender", VOTER3),
                    log("proposal_id", 1),
                ],
                data: None
            }
        )
    }

    #[test]
    fn test_vote_works() {
        let mut deps = mock_dependencies(20, &[]);

        let required_weight = 3;
        let voting_period = Duration::Time(2000000);

        let env = mock_env(OWNER, &[]);
        setup_test_case(&mut deps, env.clone(), required_weight, voting_period).unwrap();

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
        let res = handle(&mut deps, env.clone(), proposal).unwrap();

        // Get the proposal id from the logs
        let proposal_id: u64 = res.log[2].value.parse().unwrap();

        // Owner cannot vote (again)
        let vote = HandleMsg::Vote {
            proposal_id,
            vote: Vote::Yes,
        };
        let res = handle(&mut deps, env, vote.clone());

        // Verify
        assert!(res.is_err());
        match res.unwrap_err() {
            StdError::GenericErr { msg, .. } => assert_eq!(&msg, "Already voted on this proposal"),
            e => panic!("unexpected error: {}", e),
        }

        // Only voters can vote
        let env = mock_env(SOMEBODY, &[]);
        let res = handle(&mut deps, env, vote.clone());

        // Verify
        assert!(res.is_err());
        match res.unwrap_err() {
            StdError::Unauthorized { .. } => {}
            e => panic!("unexpected error: {}", e),
        }

        // But voter1 can
        let env = mock_env(VOTER1, &[]);
        let res = handle(&mut deps, env, vote.clone()).unwrap();

        // Verify
        assert_eq!(
            res,
            HandleResponse {
                messages: vec![],
                log: vec![
                    log("action", "vote"),
                    log("sender", VOTER1),
                    log("proposal_id", proposal_id),
                    log("status", "Open"),
                ],
                data: None
            }
        );

        // TODO: No/Veto votes have no effect on the tally

        // TODO: Once voted, votes cannot be changed

        // Vote it again, so it passes
        let env = mock_env(VOTER2, &[]);
        let res = handle(&mut deps, env, vote).unwrap();

        // Verify
        assert_eq!(
            res,
            HandleResponse {
                messages: vec![],
                log: vec![
                    log("action", "vote"),
                    log("sender", VOTER2),
                    log("proposal_id", proposal_id),
                    log("status", "Passed"),
                ],
                data: None
            }
        );

        // TODO: non-Open proposals cannot be voted

        // TODO: expired proposals cannot be voted
    }

    #[test]
    fn test_execute_works() {
        let mut deps = mock_dependencies(20, &[]);

        let required_weight = 3;
        let voting_period = Duration::Time(2000000);

        let env = mock_env(OWNER, &[]);
        setup_test_case(&mut deps, env.clone(), required_weight, voting_period).unwrap();

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
        let res = handle(&mut deps, env.clone(), proposal).unwrap();

        // Get the proposal id from the logs
        let proposal_id: u64 = res.log[2].value.parse().unwrap();

        // Only Passed can be executed
        let execution = HandleMsg::Execute { proposal_id };
        let res = handle(&mut deps, env.clone(), execution.clone());

        // Verify
        assert!(res.is_err());
        match res.unwrap_err() {
            StdError::GenericErr { msg, .. } => {
                assert_eq!(&msg, "Proposal must have passed and not yet been executed")
            }
            e => panic!("unexpected error: {}", e),
        }

        // Vote it, so it passes
        let vote = HandleMsg::Vote {
            proposal_id,
            vote: Vote::Yes,
        };
        let env = mock_env(VOTER3, &[]);
        let res = handle(&mut deps, env, vote).unwrap();

        // Verify
        assert_eq!(
            res,
            HandleResponse {
                messages: vec![],
                log: vec![
                    log("action", "vote"),
                    log("sender", VOTER3),
                    log("proposal_id", proposal_id),
                    log("status", "Passed"),
                ],
                data: None
            }
        );

        // Execute works. Anybody can execute Passed proposals
        let env = mock_env(SOMEBODY, &[]);
        let res = handle(&mut deps, env.clone(), execution).unwrap();

        // Verify
        assert_eq!(
            res,
            HandleResponse {
                messages: msgs,
                log: vec![
                    log("action", "execute"),
                    log("sender", SOMEBODY),
                    log("proposal_id", proposal_id),
                ],
                data: None
            }
        );
    }
}
