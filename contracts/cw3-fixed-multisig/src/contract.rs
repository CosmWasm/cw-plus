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
    use cosmwasm_std::{coin, from_binary, BankMsg};

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
        let msgs = vec![CosmosMsg::Custom(Empty {})];
        let proposal = HandleMsg::Propose {
            title: "Title".to_string(),
            description: "Description".to_string(),
            msgs,
            latest: None,
        };
        handle(&mut deps, env.clone(), proposal).unwrap();

        // Query the proposal id
        let query_msg = QueryMsg::ListProposals {
            start_after: None,
            limit: None,
        };
        let proposals: ProposalListResponse =
            from_binary(&query(&deps, query_msg).unwrap()).unwrap();
        let proposals = &proposals.proposals;
        assert_eq!(1, proposals.len());
        let proposal = &proposals[0];

        // owner cannot vote (again)
        let vote = HandleMsg::Vote {
            proposal_id: proposal.id,
            vote: Vote::Yes,
        };
        let res = handle(&mut deps, env, vote);

        // Verify
        assert!(res.is_err());
        match res.unwrap_err() {
            StdError::GenericErr { msg, .. } => assert_eq!(&msg, "Already voted on this proposal"),
            e => panic!("unexpected error: {}", e),
        }

        // But voter1 can
        let env = mock_env(VOTER1, &[]);
        let vote = HandleMsg::Vote {
            proposal_id: proposal.id,
            vote: Vote::Yes,
        };
        let res = handle(&mut deps, env, vote).unwrap();

        // Verify
        assert_eq!(
            res,
            HandleResponse {
                messages: vec![],
                log: vec![
                    log("action", "vote"),
                    log("sender", VOTER1),
                    log("proposal_id", proposal.id),
                ],
                data: None
            }
        )
    }

    /*
    #[test]
    fn increase_allowances() {
        let mut deps = mock_dependencies(20, &[]);

        let owner = HumanAddr::from("admin0001");
        let admins = vec![owner.clone(), HumanAddr::from("admin0002")];

        let spender1 = HumanAddr::from("spender0001");
        let spender2 = HumanAddr::from("spender0002");
        let spender3 = HumanAddr::from("spender0003");
        let spender4 = HumanAddr::from("spender0004");
        let initial_spenders = vec![spender1.clone(), spender2.clone()];

        // Same allowances for all spenders, for simplicity
        let denom1 = "token1";
        let denom2 = "token2";
        let denom3 = "token3";
        let amount1 = 1111;
        let amount2 = 2222;
        let amount3 = 3333;

        let allow1 = coin(amount1, denom1);
        let allow2 = coin(amount2, denom2);
        let allow3 = coin(amount3, denom3);
        let initial_allowances = vec![allow1.clone(), allow2.clone()];

        let expires_height = Expiration::AtHeight(5432);
        let expires_never = Expiration::Never {};
        let expires_time = Expiration::AtTime(1234567890);
        // Initially set first spender allowance with height expiration, the second with no expiration
        let initial_expirations = vec![expires_height.clone(), expires_never.clone()];

        let env = mock_env(owner, &[]);
        setup_test_case(
            &mut deps,
            &env,
            &admins,
            &initial_spenders,
            &initial_allowances,
            &initial_expirations,
        );

        // Add to spender1 account (expires = None) => don't change Expiration
        let msg = HandleMsg::IncreaseAllowance {
            spender: spender1.clone(),
            amount: allow1.clone(),
            expires: None,
        };
        handle(&mut deps, env.clone(), msg).unwrap();

        // Verify
        let allowance = query_allowance(&deps, spender1.clone()).unwrap();
        assert_eq!(
            allowance,
            Allowance {
                balance: NativeBalance(vec![coin(amount1 * 2, &allow1.denom), allow2.clone()]),
                expires: expires_height.clone(),
            }
        );

        // Add to spender2 account (expires = Some)
        let msg = HandleMsg::IncreaseAllowance {
            spender: spender2.clone(),
            amount: allow3.clone(),
            expires: Some(expires_height.clone()),
        };
        handle(&mut deps, env.clone(), msg).unwrap();

        // Verify
        let allowance = query_allowance(&deps, spender2.clone()).unwrap();
        assert_eq!(
            allowance,
            Allowance {
                balance: NativeBalance(vec![allow1.clone(), allow2.clone(), allow3.clone()]),
                expires: expires_height.clone(),
            }
        );

        // Add to spender3 (new account) (expires = None) => default Expiration::Never
        let msg = HandleMsg::IncreaseAllowance {
            spender: spender3.clone(),
            amount: allow1.clone(),
            expires: None,
        };
        handle(&mut deps, env.clone(), msg).unwrap();

        // Verify
        let allowance = query_allowance(&deps, spender3.clone()).unwrap();
        assert_eq!(
            allowance,
            Allowance {
                balance: NativeBalance(vec![allow1.clone()]),
                expires: expires_never.clone(),
            }
        );

        // Add to spender4 (new account) (expires = Some)
        let msg = HandleMsg::IncreaseAllowance {
            spender: spender4.clone(),
            amount: allow2.clone(),
            expires: Some(expires_time.clone()),
        };
        handle(&mut deps, env.clone(), msg).unwrap();

        // Verify
        let allowance = query_allowance(&deps, spender4.clone()).unwrap();
        assert_eq!(
            allowance,
            Allowance {
                balance: NativeBalance(vec![allow2.clone()]),
                expires: expires_time,
            }
        );
    }

    #[test]
    fn decrease_allowances() {
        let mut deps = mock_dependencies(20, &[]);

        let owner = HumanAddr::from("admin0001");
        let admins = vec![owner.clone(), HumanAddr::from("admin0002")];

        let spender1 = HumanAddr::from("spender0001");
        let spender2 = HumanAddr::from("spender0002");
        let initial_spenders = vec![spender1.clone(), spender2.clone()];

        // Same allowances for all spenders, for simplicity
        let denom1 = "token1";
        let denom2 = "token2";
        let denom3 = "token3";
        let amount1 = 1111;
        let amount2 = 2222;
        let amount3 = 3333;

        let allow1 = coin(amount1, denom1);
        let allow2 = coin(amount2, denom2);
        let allow3 = coin(amount3, denom3);

        let initial_allowances = vec![coin(amount1, denom1), coin(amount2, denom2)];

        let expires_height = Expiration::AtHeight(5432);
        let expires_never = Expiration::Never {};
        // Initially set first spender allowance with height expiration, the second with no expiration
        let initial_expirations = vec![expires_height.clone(), expires_never.clone()];

        let env = mock_env(owner, &[]);
        setup_test_case(
            &mut deps,
            &env,
            &admins,
            &initial_spenders,
            &initial_allowances,
            &initial_expirations,
        );

        // Subtract from spender1 (existing) account (has none of that denom)
        let msg = HandleMsg::DecreaseAllowance {
            spender: spender1.clone(),
            amount: allow3.clone(),
            expires: None,
        };
        let res = handle(&mut deps, env.clone(), msg);

        // Verify
        assert!(res.is_err());
        // Verify everything stays the same for that spender
        let allowance = query_allowance(&deps, spender1.clone()).unwrap();
        assert_eq!(
            allowance,
            Allowance {
                balance: NativeBalance(vec![allow1.clone(), allow2.clone()]),
                expires: expires_height.clone(),
            }
        );

        // Subtract from spender2 (existing) account (brings denom to 0, other denoms left)
        let msg = HandleMsg::DecreaseAllowance {
            spender: spender2.clone(),
            amount: allow2.clone(),
            expires: None,
        };
        handle(&mut deps, env.clone(), msg).unwrap();

        // Verify
        let allowance = query_allowance(&deps, spender2.clone()).unwrap();
        assert_eq!(
            allowance,
            Allowance {
                balance: NativeBalance(vec![allow1.clone()]),
                expires: expires_never.clone(),
            }
        );

        // Subtract from spender1 (existing) account (brings denom to > 0)
        let msg = HandleMsg::DecreaseAllowance {
            spender: spender1.clone(),
            amount: coin(amount1 / 2, denom1),
            expires: None,
        };
        handle(&mut deps, env.clone(), msg).unwrap();

        // Verify
        let allowance = query_allowance(&deps, spender1.clone()).unwrap();
        assert_eq!(
            allowance,
            Allowance {
                balance: NativeBalance(vec![
                    coin(amount1 / 2 + (amount1 & 1), denom1),
                    allow2.clone()
                ]),
                expires: expires_height.clone(),
            }
        );

        // Subtract from spender2 (existing) account (brings denom to 0, no other denoms left => should delete Allowance)
        let msg = HandleMsg::DecreaseAllowance {
            spender: spender2.clone(),
            amount: allow1.clone(),
            expires: None,
        };
        handle(&mut deps, env.clone(), msg).unwrap();

        // Verify
        let allowance = query_allowance(&deps, spender2.clone()).unwrap();
        assert_eq!(allowance, Allowance::default());

        // Subtract from spender2 (empty) account (should error)
        let msg = HandleMsg::DecreaseAllowance {
            spender: spender2.clone(),
            amount: allow1.clone(),
            expires: None,
        };
        let res = handle(&mut deps, env.clone(), msg);

        // Verify
        assert!(res.is_err());

        // Subtract from spender1 (existing) account (underflows denom => should delete denom)
        let msg = HandleMsg::DecreaseAllowance {
            spender: spender1.clone(),
            amount: coin(amount1 * 10, denom1),
            expires: None,
        };
        handle(&mut deps, env.clone(), msg).unwrap();

        // Verify
        let allowance = query_allowance(&deps, spender1.clone()).unwrap();
        assert_eq!(
            allowance,
            Allowance {
                balance: NativeBalance(vec![allow2]),
                expires: expires_height.clone(),
            }
        );
    }

    #[test]
    fn execute_checks() {
        let mut deps = mock_dependencies(20, &[]);

        let owner = HumanAddr::from("admin0001");
        let admins = vec![owner.clone(), HumanAddr::from("admin0002")];

        let spender1 = HumanAddr::from("spender0001");
        let spender2 = HumanAddr::from("spender0002");
        let initial_spenders = vec![spender1.clone()];

        let denom1 = "token1";
        let amount1 = 1111;
        let allow1 = coin(amount1, denom1);
        let initial_allowances = vec![allow1];

        let expires_never = Expiration::Never {};
        let initial_expirations = vec![expires_never.clone()];

        let env = mock_env(owner.clone(), &[]);
        setup_test_case(
            &mut deps,
            &env,
            &admins,
            &initial_spenders,
            &initial_allowances,
            &initial_expirations,
        );

        // Create Send message
        let msgs = vec![BankMsg::Send {
            from_address: HumanAddr::from(MOCK_CONTRACT_ADDR),
            to_address: spender2.clone(),
            amount: coins(1000, "token1"),
        }
            .into()];

        let handle_msg = HandleMsg::Execute { msgs: msgs.clone() };

        // spender2 cannot spend funds (no initial allowance)
        let env = mock_env(&spender2, &[]);
        let res = handle(&mut deps, env, handle_msg.clone());
        match res.unwrap_err() {
            StdError::NotFound { .. } => {}
            e => panic!("unexpected error: {}", e),
        }

        // But spender1 can (he has enough funds)
        let env = mock_env(&spender1, &[]);
        let res = handle(&mut deps, env.clone(), handle_msg.clone()).unwrap();
        assert_eq!(res.messages, msgs);
        assert_eq!(
            res.log,
            vec![log("action", "execute"), log("owner", spender1.clone())]
        );

        // And then cannot (not enough funds anymore)
        let res = handle(&mut deps, env, handle_msg.clone());
        match res.unwrap_err() {
            StdError::Underflow { .. } => {}
            e => panic!("unexpected error: {}", e),
        }

        // Owner / admins can do anything (at the contract level)
        let env = mock_env(&owner.clone(), &[]);
        let res = handle(&mut deps, env.clone(), handle_msg.clone()).unwrap();
        assert_eq!(res.messages, msgs);
        assert_eq!(
            res.log,
            vec![log("action", "execute"), log("owner", owner.clone())]
        );

        // For admins, even other message types are allowed
        let other_msgs = vec![CosmosMsg::Custom(Empty {})];
        let handle_msg = HandleMsg::Execute {
            msgs: other_msgs.clone(),
        };

        let env = mock_env(&owner, &[]);
        let res = handle(&mut deps, env, handle_msg.clone()).unwrap();
        assert_eq!(res.messages, other_msgs);
        assert_eq!(res.log, vec![log("action", "execute"), log("owner", owner)]);

        // But not for mere mortals
        let env = mock_env(&spender1, &[]);
        let res = handle(&mut deps, env, handle_msg.clone());
        match res.unwrap_err() {
            StdError::GenericErr { msg, .. } => assert_eq!(&msg, "Message type rejected"),
            e => panic!("unexpected error: {}", e),
        }
    }

    #[test]
    fn staking_permission_checks() {
        let mut deps = mock_dependencies(20, &[]);

        let owner = HumanAddr::from("admin0001");
        let admins = vec![owner.clone()];

        // spender1 has every permission to stake
        let spender1 = HumanAddr::from("spender0001");
        // spender2 do not have permission
        let spender2 = HumanAddr::from("spender0002");
        let denom = "token1";
        let amount = 10000;
        let coin1 = coin(amount, denom);

        let god_mode = Permissions {
            delegate: true,
            redelegate: true,
            undelegate: true,
            withdraw: true,
        };

        let env = mock_env(owner.clone(), &[]);
        // Init a contract with admins
        let init_msg = InitMsg {
            admins: admins.clone(),
            mutable: true,
        };
        init(&mut deps, env.clone(), init_msg).unwrap();

        let setup_perm_msg1 = HandleMsg::SetPermissions {
            spender: spender1.clone(),
            permissions: god_mode,
        };
        handle(&mut deps, env.clone(), setup_perm_msg1).unwrap();

        let setup_perm_msg2 = HandleMsg::SetPermissions {
            spender: spender2.clone(),
            // default is no permission
            permissions: Default::default(),
        };
        // default is no permission
        handle(&mut deps, env.clone(), setup_perm_msg2).unwrap();

        let msg_delegate = vec![StakingMsg::Delegate {
            validator: HumanAddr::from("validator1"),
            amount: coin1.clone(),
        }
            .into()];
        let msg_redelegate = vec![StakingMsg::Redelegate {
            src_validator: HumanAddr::from("validator1"),
            dst_validator: HumanAddr::from("validator2"),
            amount: coin1.clone(),
        }
            .into()];
        let msg_undelegate = vec![StakingMsg::Undelegate {
            validator: HumanAddr::from("validator1"),
            amount: coin1.clone(),
        }
            .into()];
        let msg_withdraw = vec![StakingMsg::Withdraw {
            validator: HumanAddr::from("validator1"),
            recipient: None,
        }
            .into()];

        let msgs = vec![
            msg_delegate.clone(),
            msg_redelegate.clone(),
            msg_undelegate.clone(),
            msg_withdraw.clone(),
        ];

        // spender1 can execute
        for msg in &msgs {
            let env = mock_env(&spender1, &[]);
            let res = handle(&mut deps, env, HandleMsg::Execute { msgs: msg.clone() });
            assert!(res.is_ok())
        }

        // spender2 cannot execute (no permission)
        for msg in &msgs {
            let env = mock_env(&spender2, &[]);
            let res = handle(&mut deps, env, HandleMsg::Execute { msgs: msg.clone() });
            assert!(res.is_err())
        }

        // test mixed permissions
        let spender3 = HumanAddr::from("spender0003");
        let setup_perm_msg3 = HandleMsg::SetPermissions {
            spender: spender3.clone(),
            permissions: Permissions {
                delegate: false,
                redelegate: true,
                undelegate: true,
                withdraw: false,
            },
        };
        handle(&mut deps, env.clone(), setup_perm_msg3).unwrap();
        let env = mock_env(&spender3, &[]);
        let res = handle(
            &mut deps,
            env.clone(),
            HandleMsg::Execute {
                msgs: msg_delegate.clone(),
            },
        );
        // FIXME need better error check here
        assert!(res.is_err());
        let res = handle(
            &mut deps,
            env.clone(),
            HandleMsg::Execute {
                msgs: msg_redelegate.clone(),
            },
        );
        assert!(res.is_ok());
        let res = handle(
            &mut deps,
            env.clone(),
            HandleMsg::Execute {
                msgs: msg_undelegate.clone(),
            },
        );
        assert!(res.is_ok());
        let res = handle(
            &mut deps,
            env.clone(),
            HandleMsg::Execute {
                msgs: msg_withdraw.clone(),
            },
        );
        assert!(res.is_err())
    }

    // tests permissions and allowances are independent features and does not affect each other
    #[test]
    fn permissions_allowances_independent() {
        let mut deps = mock_dependencies(20, &[]);

        let owner = HumanAddr::from("admin0001");
        let admins = vec![owner.clone()];

        // spender1 has every permission to stake
        let spender1 = HumanAddr::from("spender0001");
        let spender2 = HumanAddr::from("spender0002");
        let denom = "token1";
        let amount = 10000;
        let coin = coin(amount, denom);

        let allow = Allowance {
            balance: NativeBalance(vec![coin.clone()]),
            expires: Expiration::Never {},
        };
        let perm = Permissions {
            delegate: true,
            redelegate: false,
            undelegate: false,
            withdraw: true,
        };

        let env = mock_env(owner.clone(), &[]);
        // Init a contract with admins
        let init_msg = InitMsg {
            admins: admins.clone(),
            mutable: true,
        };
        init(&mut deps, env.clone(), init_msg).unwrap();

        // setup permission and then allowance and check if changed
        let setup_perm_msg = HandleMsg::SetPermissions {
            spender: spender1.clone(),
            permissions: perm,
        };
        handle(&mut deps, env.clone(), setup_perm_msg).unwrap();

        let setup_allowance_msg = HandleMsg::IncreaseAllowance {
            spender: spender1.clone(),
            amount: coin.clone(),
            expires: None,
        };
        handle(&mut deps, env.clone(), setup_allowance_msg).unwrap();

        let res_perm = query_permissions(&deps, spender1.clone()).unwrap();
        assert_eq!(perm, res_perm);
        let res_allow = query_allowance(&deps, spender1.clone()).unwrap();
        assert_eq!(allow, res_allow);

        // setup allowance and then permission and check if changed
        let setup_allowance_msg = HandleMsg::IncreaseAllowance {
            spender: spender2.clone(),
            amount: coin.clone(),
            expires: None,
        };
        handle(&mut deps, env.clone(), setup_allowance_msg).unwrap();

        let setup_perm_msg = HandleMsg::SetPermissions {
            spender: spender2.clone(),
            permissions: perm,
        };
        handle(&mut deps, env.clone(), setup_perm_msg).unwrap();

        let res_perm = query_permissions(&deps, spender2.clone()).unwrap();
        assert_eq!(perm, res_perm);
        let res_allow = query_allowance(&deps, spender2.clone()).unwrap();
        assert_eq!(allow, res_allow);
    }

    #[test]
    fn can_send_query_works() {
        let mut deps = mock_dependencies(20, &[]);

        let owner = HumanAddr::from("admin007");
        let spender = HumanAddr::from("spender808");
        let anyone = HumanAddr::from("anyone");

        let env = mock_env(owner.clone(), &[]);
        // spender has allowance of 55000 ushell
        setup_test_case(
            &mut deps,
            &env,
            &[owner.clone()],
            &[spender.clone()],
            &coins(55000, "ushell"),
            &[Expiration::Never {}],
        );

        let perm = Permissions {
            delegate: true,
            redelegate: true,
            undelegate: false,
            withdraw: false,
        };

        let spender_raw = &deps.api.canonical_address(&spender).unwrap();
        let _ = permissions(&mut deps.storage).save(spender_raw.as_slice(), &perm);

        // let us make some queries... different msg types by owner and by other
        let send_msg = CosmosMsg::Bank(BankMsg::Send {
            from_address: MOCK_CONTRACT_ADDR.into(),
            to_address: anyone.clone(),
            amount: coins(12345, "ushell"),
        });
        let send_msg_large = CosmosMsg::Bank(BankMsg::Send {
            from_address: MOCK_CONTRACT_ADDR.into(),
            to_address: anyone.clone(),
            amount: coins(1234567, "ushell"),
        });
        let staking_delegate_msg = CosmosMsg::Staking(StakingMsg::Delegate {
            validator: anyone.clone(),
            amount: coin(70000, "ureef"),
        });
        let staking_withdraw_msg = CosmosMsg::Staking(StakingMsg::Withdraw {
            validator: anyone.clone(),
            recipient: None,
        });

        // owner can send big or small
        let res = query_can_send(&deps, owner.clone(), send_msg.clone()).unwrap();
        assert_eq!(res.can_send, true);
        let res = query_can_send(&deps, owner.clone(), send_msg_large.clone()).unwrap();
        assert_eq!(res.can_send, true);
        // owner can stake
        let res = query_can_send(&deps, owner.clone(), staking_delegate_msg.clone()).unwrap();
        assert_eq!(res.can_send, true);

        // spender can send small
        let res = query_can_send(&deps, spender.clone(), send_msg.clone()).unwrap();
        assert_eq!(res.can_send, true);
        // not too big
        let res = query_can_send(&deps, spender.clone(), send_msg_large.clone()).unwrap();
        assert_eq!(res.can_send, false);
        // spender can send staking msgs if permissioned
        let res = query_can_send(&deps, spender.clone(), staking_delegate_msg.clone()).unwrap();
        assert_eq!(res.can_send, true);
        let res = query_can_send(&deps, spender.clone(), staking_withdraw_msg.clone()).unwrap();
        assert_eq!(res.can_send, false);

        // random person cannot do anything
        let res = query_can_send(&deps, anyone.clone(), send_msg.clone()).unwrap();
        assert_eq!(res.can_send, false);
        let res = query_can_send(&deps, anyone.clone(), send_msg_large.clone()).unwrap();
        assert_eq!(res.can_send, false);
        let res = query_can_send(&deps, anyone.clone(), staking_delegate_msg.clone()).unwrap();
        assert_eq!(res.can_send, false);
        let res = query_can_send(&deps, anyone.clone(), staking_withdraw_msg.clone()).unwrap();
        assert_eq!(res.can_send, false);
    }
    */
}
