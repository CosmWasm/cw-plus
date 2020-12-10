use std::cmp::Ordering;

use cosmwasm_std::{
    attr, to_binary, Binary, BlockInfo, CanonicalAddr, CosmosMsg, Deps, DepsMut, Empty, Env,
    HandleResponse, HumanAddr, InitResponse, MessageInfo, Order, StdResult,
};

use cw0::{maybe_canonical, Expiration};
use cw2::set_contract_version;
use cw3::{
    ProposalListResponse, ProposalResponse, Status, ThresholdResponse, Vote, VoteInfo,
    VoteListResponse, VoteResponse, VoterInfo, VoterListResponse, VoterResponse,
};
use cw4::{Cw4Contract, MemberChangedHookMsg, MemberDiff};
use cw_storage_plus::Bound;

use crate::error::ContractError;
use crate::msg::{HandleMsg, InitMsg, QueryMsg};
use crate::state::{next_id, parse_id, Ballot, Config, Proposal, BALLOTS, CONFIG, PROPOSALS};

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
    if deps.api.canonical_address(&msg.group_addr).is_err() {
        return Err(ContractError::InvalidGroup {
            addr: msg.group_addr,
        });
    }

    let group = Cw4Contract(msg.group_addr);
    let total_weight = group.total_weight(&deps.querier)?;

    if total_weight < msg.required_weight {
        return Err(ContractError::UnreachableWeight {});
    }

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let cfg = Config {
        required_weight: msg.required_weight,
        max_voting_period: msg.max_voting_period,
        group_addr: group,
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
        HandleMsg::MemberChangedHook(MemberChangedHookMsg { diffs }) => {
            handle_membership_hook(deps, env, info, diffs)
        }
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
        .group_addr
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
        start_height: env.block.height,
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

    // ensure proposal exists and can be voted on
    let mut prop = PROPOSALS.load(deps.storage, proposal_id.into())?;
    if prop.status != Status::Open {
        return Err(ContractError::NotOpen {});
    }
    if prop.expires.is_expired(&env.block) {
        return Err(ContractError::Expired {});
    }

    // use a snapshot of "start of proposal" if available, otherwise, current group weight
    let vote_power = cfg
        .group_addr
        .member_at_height(&deps.querier, info.sender.clone(), prop.start_height)?
        .ok_or_else(|| ContractError::Unauthorized {})?;

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

pub fn handle_membership_hook(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    _diffs: Vec<MemberDiff>,
) -> Result<HandleResponse<Empty>, ContractError> {
    // This is now a no-op
    // But we leave the authorization check as a demo
    let cfg = CONFIG.load(deps.storage)?;
    if info.sender != cfg.group_addr.0 {
        return Err(ContractError::Unauthorized {});
    }

    Ok(HandleResponse::default())
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
    let total_weight = cfg.group_addr.total_weight(&deps.querier)?;
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

fn query_voter(deps: Deps, voter: HumanAddr) -> StdResult<VoterInfo> {
    let cfg = CONFIG.load(deps.storage)?;
    let voter_raw = deps.api.canonical_address(&voter)?;
    let weight = cfg.group_addr.is_member(&deps.querier, &voter_raw)?;

    Ok(VoterInfo { weight })
}

fn list_voters(
    deps: Deps,
    start_after: Option<HumanAddr>,
    limit: Option<u32>,
) -> StdResult<VoterListResponse> {
    let cfg = CONFIG.load(deps.storage)?;
    let voters = cfg
        .group_addr
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
    use cosmwasm_std::testing::{mock_env, MockApi, MockStorage};
    use cosmwasm_std::{coin, coins, BankMsg, Coin};

    use cw0::Duration;
    use cw2::{query_contract_info, ContractVersion};
    use cw4::{Cw4HandleMsg, Member};
    use cw_multi_test::{next_block, App, Contract, ContractWrapper, SimpleBank};

    use super::*;

    const OWNER: &str = "admin0001";
    const VOTER1: &str = "voter0001";
    const VOTER2: &str = "voter0002";
    const VOTER3: &str = "voter0003";
    const VOTER4: &str = "voter0004";
    const VOTER5: &str = "voter0005";
    const SOMEBODY: &str = "somebody";

    fn member<T: Into<HumanAddr>>(addr: T, weight: u64) -> Member {
        Member {
            addr: addr.into(),
            weight,
        }
    }

    pub fn contract_flex() -> Box<dyn Contract> {
        let contract = ContractWrapper::new(
            crate::contract::handle,
            crate::contract::init,
            crate::contract::query,
        );
        Box::new(contract)
    }

    pub fn contract_group() -> Box<dyn Contract> {
        let contract = ContractWrapper::new(
            cw4_group::contract::handle,
            cw4_group::contract::init,
            cw4_group::contract::query,
        );
        Box::new(contract)
    }

    fn mock_app() -> App {
        let env = mock_env();
        let api = Box::new(MockApi::default());
        let bank = SimpleBank {};

        App::new(api, env.block, bank, || Box::new(MockStorage::new()))
    }

    // uploads code and returns address of group contract
    fn init_group(app: &mut App, members: Vec<Member>) -> HumanAddr {
        let group_id = app.store_code(contract_group());
        let msg = cw4_group::msg::InitMsg {
            admin: Some(OWNER.into()),
            members,
        };
        app.instantiate_contract(group_id, OWNER, &msg, &[], "group")
            .unwrap()
    }

    // uploads code and returns address of group contract
    fn init_flex(
        app: &mut App,
        group: HumanAddr,
        required_weight: u64,
        max_voting_period: Duration,
    ) -> HumanAddr {
        let flex_id = app.store_code(contract_flex());
        let msg = crate::msg::InitMsg {
            group_addr: group,
            required_weight,
            max_voting_period,
        };
        app.instantiate_contract(flex_id, OWNER, &msg, &[], "flex")
            .unwrap()
    }

    // this will set up both contracts, initializing the group with
    // all voters defined above, and the multisig pointing to it and given threshold criteria.
    // Returns (multisig address, group address).
    fn setup_test_case(
        app: &mut App,
        required_weight: u64,
        max_voting_period: Duration,
        init_funds: Vec<Coin>,
        multisig_as_group_admin: bool,
    ) -> (HumanAddr, HumanAddr) {
        // 1. Initialize group contract with members (and OWNER as admin)
        let members = vec![
            member(OWNER, 0),
            member(VOTER1, 1),
            member(VOTER2, 2),
            member(VOTER3, 3),
            member(VOTER4, 4),
            member(VOTER5, 5),
        ];
        let group_addr = init_group(app, members);
        app.update_block(next_block);

        // 2. Set up Multisig backed by this group
        let flex_addr = init_flex(app, group_addr.clone(), required_weight, max_voting_period);
        app.update_block(next_block);

        // 3. (Optional) Set the multisig as the group owner
        if multisig_as_group_admin {
            let update_admin = Cw4HandleMsg::UpdateAdmin {
                admin: Some(flex_addr.clone()),
            };
            app.execute_contract(OWNER, &group_addr, &update_admin, &[])
                .unwrap();
            app.update_block(next_block);
        }

        // Bonus: set some funds on the multisig contract for future proposals
        if !init_funds.is_empty() {
            app.set_bank_balance(flex_addr.clone(), init_funds).unwrap();
        }
        (flex_addr, group_addr)
    }

    fn pay_somebody_proposal(flex_addr: &HumanAddr) -> HandleMsg {
        let bank_msg = BankMsg::Send {
            from_address: flex_addr.clone(),
            to_address: SOMEBODY.into(),
            amount: coins(1, "BTC"),
        };
        let msgs = vec![CosmosMsg::Bank(bank_msg)];
        HandleMsg::Propose {
            title: "Pay somebody".to_string(),
            description: "Do I pay her?".to_string(),
            msgs,
            latest: None,
        }
    }

    #[test]
    fn test_init_works() {
        let mut app = mock_app();

        // make a simple group
        let group_addr = init_group(&mut app, vec![member(OWNER, 1)]);
        let flex_id = app.store_code(contract_flex());

        let max_voting_period = Duration::Time(1234567);

        // Zero required weight fails
        let init_msg = InitMsg {
            group_addr: group_addr.clone(),
            required_weight: 0,
            max_voting_period,
        };
        let res = app.instantiate_contract(flex_id, OWNER, &init_msg, &[], "zero required weight");

        // Verify
        assert_eq!(res.unwrap_err(), ContractError::ZeroWeight {}.to_string());

        // Total weight less than required weight not allowed
        let init_msg = InitMsg {
            group_addr: group_addr.clone(),
            required_weight: 100,
            max_voting_period,
        };
        let res = app.instantiate_contract(flex_id, OWNER, &init_msg, &[], "high required weight");

        // Verify
        assert_eq!(
            res.unwrap_err(),
            ContractError::UnreachableWeight {}.to_string()
        );

        // All valid
        let init_msg = InitMsg {
            group_addr: group_addr.clone(),
            required_weight: 1,
            max_voting_period,
        };
        let flex_addr = app
            .instantiate_contract(flex_id, OWNER, &init_msg, &[], "all good")
            .unwrap();

        // Verify contract version set properly
        let version = query_contract_info(&app, &flex_addr).unwrap();
        assert_eq!(
            ContractVersion {
                contract: CONTRACT_NAME.to_string(),
                version: CONTRACT_VERSION.to_string(),
            },
            version,
        );

        // Get voters query
        let voters: VoterListResponse = app
            .wrap()
            .query_wasm_smart(
                &flex_addr,
                &QueryMsg::ListVoters {
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap();
        assert_eq!(
            voters.voters,
            vec![VoterResponse {
                addr: OWNER.into(),
                weight: 1
            }]
        );
    }

    #[test]
    fn test_propose_works() {
        let mut app = mock_app();

        let required_weight = 4;
        let voting_period = Duration::Time(2000000);
        let (flex_addr, _) = setup_test_case(
            &mut app,
            required_weight,
            voting_period,
            coins(10, "BTC"),
            false,
        );

        let proposal = pay_somebody_proposal(&flex_addr);
        // Only voters can propose
        let res = app.execute_contract(SOMEBODY, &flex_addr, &proposal, &[]);
        assert_eq!(res.unwrap_err(), ContractError::Unauthorized {}.to_string());

        // Wrong expiration option fails
        let msgs = match proposal.clone() {
            HandleMsg::Propose { msgs, .. } => msgs,
            _ => panic!("Wrong variant"),
        };
        let proposal_wrong_exp = HandleMsg::Propose {
            title: "Rewarding somebody".to_string(),
            description: "Do we reward her?".to_string(),
            msgs,
            latest: Some(Expiration::AtHeight(123456)),
        };
        let res = app.execute_contract(OWNER, &flex_addr, &proposal_wrong_exp, &[]);
        assert_eq!(
            res.unwrap_err(),
            ContractError::WrongExpiration {}.to_string()
        );

        // Proposal from voter works
        let res = app
            .execute_contract(VOTER3, &flex_addr, &proposal, &[])
            .unwrap();
        assert_eq!(
            res.attributes,
            vec![
                attr("action", "propose"),
                attr("sender", VOTER3),
                attr("proposal_id", 1),
                attr("status", "Open"),
            ],
        );

        // Proposal from voter with enough vote power directly passes
        let res = app
            .execute_contract(VOTER4, &flex_addr, &proposal, &[])
            .unwrap();
        assert_eq!(
            res.attributes,
            vec![
                attr("action", "propose"),
                attr("sender", VOTER4),
                attr("proposal_id", 2),
                attr("status", "Passed"),
            ],
        );
    }

    fn get_tally(app: &App, flex_addr: &HumanAddr, proposal_id: u64) -> u64 {
        // Get all the voters on the proposal
        let voters = QueryMsg::ListVotes {
            proposal_id,
            start_after: None,
            limit: None,
        };
        let votes: VoteListResponse = app.wrap().query_wasm_smart(flex_addr, &voters).unwrap();
        // Sum the weights of the Yes votes to get the tally
        votes
            .votes
            .iter()
            .filter(|&v| v.vote == Vote::Yes)
            .map(|v| v.weight)
            .sum()
    }

    fn expire(voting_period: Duration) -> impl Fn(&mut BlockInfo) {
        move |block: &mut BlockInfo| {
            match voting_period {
                Duration::Time(duration) => block.time += duration + 1,
                Duration::Height(duration) => block.height += duration + 1,
            };
        }
    }

    fn unexpire(voting_period: Duration) -> impl Fn(&mut BlockInfo) {
        move |block: &mut BlockInfo| {
            match voting_period {
                Duration::Time(duration) => block.time -= duration,
                Duration::Height(duration) => block.height -= duration,
            };
        }
    }

    #[test]
    fn test_vote_works() {
        let mut app = mock_app();

        let required_weight = 3;
        let voting_period = Duration::Time(2000000);
        let (flex_addr, _) = setup_test_case(
            &mut app,
            required_weight,
            voting_period,
            coins(10, "BTC"),
            false,
        );

        // create proposal with 0 vote power
        let proposal = pay_somebody_proposal(&flex_addr);
        let res = app
            .execute_contract(OWNER, &flex_addr, &proposal, &[])
            .unwrap();

        // Get the proposal id from the logs
        let proposal_id: u64 = res.attributes[2].value.parse().unwrap();

        // Owner cannot vote (again)
        let yes_vote = HandleMsg::Vote {
            proposal_id,
            vote: Vote::Yes,
        };
        let err = app
            .execute_contract(OWNER, &flex_addr, &yes_vote, &[])
            .unwrap_err();
        assert_eq!(err, ContractError::AlreadyVoted {}.to_string());

        // Only voters can vote
        let err = app
            .execute_contract(SOMEBODY, &flex_addr, &yes_vote, &[])
            .unwrap_err();
        assert_eq!(err, ContractError::Unauthorized {}.to_string());

        // But voter1 can
        let res = app
            .execute_contract(VOTER1, &flex_addr, &yes_vote, &[])
            .unwrap();
        assert_eq!(
            res.attributes,
            vec![
                attr("action", "vote"),
                attr("sender", VOTER1),
                attr("proposal_id", proposal_id),
                attr("status", "Open"),
            ],
        );

        // No/Veto votes have no effect on the tally
        // Compute the current tally
        let tally = get_tally(&app, &flex_addr, proposal_id);
        assert_eq!(tally, 1);

        // Cast a No vote
        let no_vote = HandleMsg::Vote {
            proposal_id,
            vote: Vote::No,
        };
        let _ = app
            .execute_contract(VOTER2, &flex_addr, &no_vote, &[])
            .unwrap();

        // Cast a Veto vote
        let veto_vote = HandleMsg::Vote {
            proposal_id,
            vote: Vote::Veto,
        };
        let _ = app
            .execute_contract(VOTER3, &flex_addr, &veto_vote, &[])
            .unwrap();

        // Tally unchanged
        assert_eq!(tally, get_tally(&app, &flex_addr, proposal_id));

        let err = app
            .execute_contract(VOTER3, &flex_addr, &yes_vote, &[])
            .unwrap_err();
        assert_eq!(err, ContractError::AlreadyVoted {}.to_string());

        // Expired proposals cannot be voted
        app.update_block(expire(voting_period));
        let err = app
            .execute_contract(VOTER4, &flex_addr, &yes_vote, &[])
            .unwrap_err();
        assert_eq!(err, ContractError::Expired {}.to_string());
        app.update_block(unexpire(voting_period));

        // Powerful voter supports it, so it passes
        let res = app
            .execute_contract(VOTER4, &flex_addr, &yes_vote, &[])
            .unwrap();
        assert_eq!(
            res.attributes,
            vec![
                attr("action", "vote"),
                attr("sender", VOTER4),
                attr("proposal_id", proposal_id),
                attr("status", "Passed"),
            ],
        );

        // non-Open proposals cannot be voted
        let err = app
            .execute_contract(VOTER5, &flex_addr, &yes_vote, &[])
            .unwrap_err();
        assert_eq!(err, ContractError::NotOpen {}.to_string());
    }

    #[test]
    fn test_execute_works() {
        let mut app = mock_app();

        let required_weight = 3;
        let voting_period = Duration::Time(2000000);
        let (flex_addr, _) = setup_test_case(
            &mut app,
            required_weight,
            voting_period,
            coins(10, "BTC"),
            true,
        );

        // ensure we have cash to cover the proposal
        let contract_bal = app.wrap().query_balance(&flex_addr, "BTC").unwrap();
        assert_eq!(contract_bal, coin(10, "BTC"));

        // create proposal with 0 vote power
        let proposal = pay_somebody_proposal(&flex_addr);
        let res = app
            .execute_contract(OWNER, &flex_addr, &proposal, &[])
            .unwrap();

        // Get the proposal id from the logs
        let proposal_id: u64 = res.attributes[2].value.parse().unwrap();

        // Only Passed can be executed
        let execution = HandleMsg::Execute { proposal_id };
        let err = app
            .execute_contract(OWNER, &flex_addr, &execution, &[])
            .unwrap_err();
        assert_eq!(err, ContractError::WrongExecuteStatus {}.to_string());

        // Vote it, so it passes
        let vote = HandleMsg::Vote {
            proposal_id,
            vote: Vote::Yes,
        };
        let res = app
            .execute_contract(VOTER3, &flex_addr, &vote, &[])
            .unwrap();
        assert_eq!(
            res.attributes,
            vec![
                attr("action", "vote"),
                attr("sender", VOTER3),
                attr("proposal_id", proposal_id),
                attr("status", "Passed"),
            ],
        );

        // In passing: Try to close Passed fails
        let closing = HandleMsg::Close { proposal_id };
        let err = app
            .execute_contract(OWNER, &flex_addr, &closing, &[])
            .unwrap_err();
        assert_eq!(err, ContractError::WrongCloseStatus {}.to_string());

        // Execute works. Anybody can execute Passed proposals
        let res = app
            .execute_contract(SOMEBODY, &flex_addr, &execution, &[])
            .unwrap();
        assert_eq!(
            res.attributes,
            vec![
                attr("action", "execute"),
                attr("sender", SOMEBODY),
                attr("proposal_id", proposal_id),
            ],
        );

        // verify money was transfered
        let some_bal = app.wrap().query_balance(SOMEBODY, "BTC").unwrap();
        assert_eq!(some_bal, coin(1, "BTC"));
        let contract_bal = app.wrap().query_balance(&flex_addr, "BTC").unwrap();
        assert_eq!(contract_bal, coin(9, "BTC"));

        // In passing: Try to close Executed fails
        let err = app
            .execute_contract(OWNER, &flex_addr, &closing, &[])
            .unwrap_err();
        assert_eq!(err, ContractError::WrongCloseStatus {}.to_string());
    }

    #[test]
    fn test_close_works() {
        let mut app = mock_app();

        let required_weight = 3;
        let voting_period = Duration::Height(2000000);
        let (flex_addr, _) = setup_test_case(
            &mut app,
            required_weight,
            voting_period,
            coins(10, "BTC"),
            true,
        );

        // create proposal with 0 vote power
        let proposal = pay_somebody_proposal(&flex_addr);
        let res = app
            .execute_contract(OWNER, &flex_addr, &proposal, &[])
            .unwrap();

        // Get the proposal id from the logs
        let proposal_id: u64 = res.attributes[2].value.parse().unwrap();

        // Non-expired proposals cannot be closed
        let closing = HandleMsg::Close { proposal_id };
        let err = app
            .execute_contract(SOMEBODY, &flex_addr, &closing, &[])
            .unwrap_err();
        assert_eq!(err, ContractError::NotExpired {}.to_string());

        // Expired proposals can be closed
        app.update_block(expire(voting_period));
        let res = app
            .execute_contract(SOMEBODY, &flex_addr, &closing, &[])
            .unwrap();
        assert_eq!(
            res.attributes,
            vec![
                attr("action", "close"),
                attr("sender", SOMEBODY),
                attr("proposal_id", proposal_id),
            ],
        );

        // Trying to close it again fails
        let closing = HandleMsg::Close { proposal_id };
        let err = app
            .execute_contract(SOMEBODY, &flex_addr, &closing, &[])
            .unwrap_err();
        assert_eq!(err, ContractError::WrongCloseStatus {}.to_string());
    }

    // uses the power from the beginning of the voting period
    #[test]
    fn handle_group_changes_from_external() {
        let mut app = mock_app();

        let required_weight = 4;
        let voting_period = Duration::Time(20000);
        let (flex_addr, group_addr) = setup_test_case(
            &mut app,
            required_weight,
            voting_period,
            coins(10, "BTC"),
            false,
        );

        // VOTER1 starts a proposal to send some tokens (1/4 votes)
        let proposal = pay_somebody_proposal(&flex_addr);
        let res = app
            .execute_contract(VOTER1, &flex_addr, &proposal, &[])
            .unwrap();
        // Get the proposal id from the logs
        let proposal_id: u64 = res.attributes[2].value.parse().unwrap();
        let prop_status = |app: &App, proposal_id: u64| -> Status {
            let query_prop = QueryMsg::Proposal { proposal_id };
            let prop: ProposalResponse = app
                .wrap()
                .query_wasm_smart(&flex_addr, &query_prop)
                .unwrap();
            prop.status
        };

        // 1/4 votes
        assert_eq!(prop_status(&app, proposal_id), Status::Open);

        // a few blocks later...
        app.update_block(|block| block.height += 2);

        // admin changes the group
        // updates VOTER2 power to 7 -> with snapshot, vote doesn't pass proposal
        // adds NEWBIE with 2 power -> with snapshot, invalid vote
        // removes VOTER3 -> with snapshot, can vote and pass proposal
        let newbie: &str = "newbie";
        let update_msg = Cw4HandleMsg::UpdateMembers {
            remove: vec![VOTER3.into()],
            add: vec![member(VOTER2, 7), member(newbie, 2)],
        };
        app.execute_contract(OWNER, &group_addr, &update_msg, &[])
            .unwrap();

        // check membership queries properly updated
        let query_voter = QueryMsg::Voter {
            address: VOTER3.into(),
        };
        let power: VoterInfo = app
            .wrap()
            .query_wasm_smart(&flex_addr, &query_voter)
            .unwrap();
        assert_eq!(power.weight, None);

        // proposal still open
        assert_eq!(prop_status(&app, proposal_id), Status::Open);

        // a few blocks later...
        app.update_block(|block| block.height += 3);

        // make a second proposal
        let proposal2 = pay_somebody_proposal(&flex_addr);
        let res = app
            .execute_contract(VOTER1, &flex_addr, &proposal2, &[])
            .unwrap();
        // Get the proposal id from the logs
        let proposal_id2: u64 = res.attributes[2].value.parse().unwrap();

        // VOTER2 can pass this alone with the updated vote (newer height ignores snapshot)
        let yes_vote = HandleMsg::Vote {
            proposal_id: proposal_id2,
            vote: Vote::Yes,
        };
        app.execute_contract(VOTER2, &flex_addr, &yes_vote, &[])
            .unwrap();
        assert_eq!(prop_status(&app, proposal_id2), Status::Passed);

        // VOTER2 can only vote on first proposal with weight of 2 (not enough to pass)
        let yes_vote = HandleMsg::Vote {
            proposal_id,
            vote: Vote::Yes,
        };
        app.execute_contract(VOTER2, &flex_addr, &yes_vote, &[])
            .unwrap();
        assert_eq!(prop_status(&app, proposal_id), Status::Open);

        // newbie cannot vote
        let err = app
            .execute_contract(newbie, &flex_addr, &yes_vote, &[])
            .unwrap_err();
        assert_eq!(err, ContractError::Unauthorized {}.to_string());

        // previously removed VOTER3 can still vote, passing the proposal
        app.execute_contract(VOTER3, &flex_addr, &yes_vote, &[])
            .unwrap();
        assert_eq!(prop_status(&app, proposal_id), Status::Passed);
    }

    // uses the power from the beginning of the voting period
    // similar to above - simpler case, but shows that one proposals can
    // trigger the action
    #[test]
    fn handle_group_changes_from_proposal() {
        let mut app = mock_app();

        let required_weight = 4;
        let voting_period = Duration::Time(20000);
        let (flex_addr, group_addr) = setup_test_case(
            &mut app,
            required_weight,
            voting_period,
            coins(10, "BTC"),
            true,
        );

        // Start a proposal to remove VOTER3 from the set
        let update_msg = Cw4Contract(group_addr.clone())
            .update_members(vec![VOTER3.into()], vec![])
            .unwrap();
        let update_proposal = HandleMsg::Propose {
            title: "Kick out VOTER3".to_string(),
            description: "He's trying to steal our money".to_string(),
            msgs: vec![update_msg],
            latest: None,
        };
        let res = app
            .execute_contract(VOTER1, &flex_addr, &update_proposal, &[])
            .unwrap();
        // Get the proposal id from the logs
        let update_proposal_id: u64 = res.attributes[2].value.parse().unwrap();

        // next block...
        app.update_block(|b| b.height += 1);

        // VOTER1 starts a proposal to send some tokens
        let cash_proposal = pay_somebody_proposal(&flex_addr);
        let res = app
            .execute_contract(VOTER1, &flex_addr, &cash_proposal, &[])
            .unwrap();
        // Get the proposal id from the logs
        let cash_proposal_id: u64 = res.attributes[2].value.parse().unwrap();
        assert_ne!(cash_proposal_id, update_proposal_id);

        // query proposal state
        let prop_status = |app: &App, proposal_id: u64| -> Status {
            let query_prop = QueryMsg::Proposal { proposal_id };
            let prop: ProposalResponse = app
                .wrap()
                .query_wasm_smart(&flex_addr, &query_prop)
                .unwrap();
            prop.status
        };
        assert_eq!(prop_status(&app, cash_proposal_id), Status::Open);
        assert_eq!(prop_status(&app, update_proposal_id), Status::Open);

        // next block...
        app.update_block(|b| b.height += 1);

        // Pass and execute first proposal
        let yes_vote = HandleMsg::Vote {
            proposal_id: update_proposal_id,
            vote: Vote::Yes,
        };
        app.execute_contract(VOTER4, &flex_addr, &yes_vote, &[])
            .unwrap();
        let execution = HandleMsg::Execute {
            proposal_id: update_proposal_id,
        };
        app.execute_contract(VOTER4, &flex_addr, &execution, &[])
            .unwrap();

        // ensure that the update_proposal is executed, but the other unchanged
        assert_eq!(prop_status(&app, update_proposal_id), Status::Executed);
        assert_eq!(prop_status(&app, cash_proposal_id), Status::Open);

        // next block...
        app.update_block(|b| b.height += 1);

        // VOTER3 can still pass the cash proposal
        // voting on it fails
        let yes_vote = HandleMsg::Vote {
            proposal_id: cash_proposal_id,
            vote: Vote::Yes,
        };
        app.execute_contract(VOTER3, &flex_addr, &yes_vote, &[])
            .unwrap();
        assert_eq!(prop_status(&app, cash_proposal_id), Status::Passed);

        // but cannot open a new one
        let cash_proposal = pay_somebody_proposal(&flex_addr);
        let err = app
            .execute_contract(VOTER3, &flex_addr, &cash_proposal, &[])
            .unwrap_err();
        assert_eq!(err, ContractError::Unauthorized {}.to_string());

        // extra: ensure no one else can call the hook
        let hook_hack = HandleMsg::MemberChangedHook(MemberChangedHookMsg {
            diffs: vec![MemberDiff::new(VOTER1, Some(1), None)],
        });
        let err = app
            .execute_contract(VOTER2, &flex_addr, &hook_hack, &[])
            .unwrap_err();
        assert_eq!(err, ContractError::Unauthorized {}.to_string());
    }
}
