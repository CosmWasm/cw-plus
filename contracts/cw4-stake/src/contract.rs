use cosmwasm_std::{
    coin, coins, to_binary, Api, BankMsg, Binary, CanonicalAddr, CosmosMsg, Deps, DepsMut, Env,
    HandleResponse, HumanAddr, InitResponse, MessageInfo, Order, StdResult, Storage, Uint128,
};
use cw0::{
    hooks::{add_hook, remove_hook, HOOKS},
    maybe_canonical,
};
use cw2::set_contract_version;
use cw4::{
    AdminResponse, HooksResponse, Member, MemberChangedHookMsg, MemberDiff, MemberListResponse,
    MemberResponse, TotalWeightResponse,
};
use cw_storage_plus::Bound;

use crate::error::ContractError;
use crate::msg::{ClaimsResponse, HandleMsg, InitMsg, QueryMsg, StakedResponse};
use crate::state::{Config, ADMIN, CONFIG, MEMBERS, STAKE, TOTAL};
use cw0::claim::{claim_tokens, create_claim, CLAIMS};
use cw0::hooks::prepare_hooks;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cw4-group";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

// Note, you can use StdResult in some functions where you do not
// make use of the custom errors
pub fn init(deps: DepsMut, _env: Env, _info: MessageInfo, msg: InitMsg) -> StdResult<InitResponse> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let admin_raw = maybe_canonical(deps.api, msg.admin)?;
    ADMIN.save(deps.storage, &admin_raw)?;

    // min_bond is at least 1, so 0 stake -> non-membership
    let min_bond = match msg.min_bond {
        Uint128(0) => Uint128(1),
        v => v,
    };

    let config = Config {
        denom: msg.stake,
        tokens_per_weight: msg.tokens_per_weight,
        min_bond,
        unbonding_period: msg.unbonding_period,
    };
    CONFIG.save(deps.storage, &config)?;
    TOTAL.save(deps.storage, &0)?;

    Ok(InitResponse::default())
}

// And declare a custom Error variant for the ones where you will want to make use of it
pub fn handle(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: HandleMsg,
) -> Result<HandleResponse, ContractError> {
    match msg {
        HandleMsg::UpdateAdmin { admin } => handle_update_admin(deps, info, admin),
        HandleMsg::AddHook { addr } => handle_add_hook(deps, info, addr),
        HandleMsg::RemoveHook { addr } => handle_remove_hook(deps, info, addr),
        HandleMsg::Bond {} => handle_bond(deps, env, info),
        HandleMsg::Unbond { amount } => handle_unbond(deps, env, info, amount),
        HandleMsg::Claim {} => handle_claim(deps, env, info),
    }
}

pub fn handle_update_admin(
    deps: DepsMut,
    info: MessageInfo,
    new_admin: Option<HumanAddr>,
) -> Result<HandleResponse, ContractError> {
    update_admin(deps, info.sender, new_admin)?;
    Ok(HandleResponse::default())
}

// the logic from handle_update_admin extracted for easier import
pub fn update_admin(
    deps: DepsMut,
    sender: HumanAddr,
    new_admin: Option<HumanAddr>,
) -> Result<Option<CanonicalAddr>, ContractError> {
    let api = deps.api;
    ADMIN.update(deps.storage, |state| -> Result<_, ContractError> {
        assert_admin(api, &sender, state)?;
        let new_admin = maybe_canonical(api, new_admin)?;
        Ok(new_admin)
    })
}

pub fn handle_bond(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<HandleResponse, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;

    // ensure the sent denom was proper
    // NOTE: those clones are not needed (if we move denom, we return early),
    // but the compiler cannot see that
    let sent = match info.sent_funds.len() {
        0 => Err(ContractError::MissingDenom(cfg.denom.clone())),
        1 => {
            if info.sent_funds[0].denom == cfg.denom {
                Ok(info.sent_funds[0].amount)
            } else {
                Err(ContractError::ExtraDenoms(cfg.denom.clone()))
            }
        }
        _ => Err(ContractError::ExtraDenoms(cfg.denom.clone())),
    }?;

    // update the sender's stake
    let sender_raw = deps.api.canonical_address(&info.sender)?;
    let new_stake = STAKE.update(deps.storage, &sender_raw, |stake| -> StdResult<_> {
        Ok(stake.unwrap_or_default() + sent)
    })?;

    let messages = update_membership(
        deps.storage,
        info.sender,
        &sender_raw,
        new_stake,
        &cfg,
        env.block.height,
    )?;

    Ok(HandleResponse {
        messages,
        attributes: vec![],
        data: None,
    })
}

pub fn handle_unbond(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Uint128,
) -> Result<HandleResponse, ContractError> {
    // reduce the sender's stake - aborting if insufficient
    let sender_raw = deps.api.canonical_address(&info.sender)?;
    let new_stake = STAKE.update(deps.storage, &sender_raw, |stake| -> StdResult<_> {
        stake.unwrap_or_default() - amount
    })?;

    // provide them a claim
    let cfg = CONFIG.load(deps.storage)?;
    create_claim(
        deps.storage,
        &sender_raw,
        amount,
        cfg.unbonding_period.after(&env.block),
    )?;

    let messages = update_membership(
        deps.storage,
        info.sender,
        &sender_raw,
        new_stake,
        &cfg,
        env.block.height,
    )?;

    Ok(HandleResponse {
        messages,
        attributes: vec![],
        data: None,
    })
}

fn update_membership(
    storage: &mut dyn Storage,
    sender: HumanAddr,
    sender_raw: &CanonicalAddr,
    new_stake: Uint128,
    cfg: &Config,
    height: u64,
) -> StdResult<Vec<CosmosMsg>> {
    // update their membership weight
    let new = calc_weight(new_stake, cfg);
    let old = MEMBERS.may_load(storage, sender_raw)?;
    match new.as_ref() {
        Some(w) => MEMBERS.save(storage, sender_raw, w, height),
        None => MEMBERS.remove(storage, sender_raw, height),
    }?;

    // update total
    TOTAL.update(storage, |total| -> StdResult<_> {
        Ok(total + new.unwrap_or_default() - old.unwrap_or_default())
    })?;

    // alert the hooks
    let diff = MemberDiff::new(sender, old, new);
    prepare_hooks(storage, |h| {
        MemberChangedHookMsg::one(diff.clone()).into_cosmos_msg(h)
    })
}

fn calc_weight(stake: Uint128, cfg: &Config) -> Option<u64> {
    if stake < cfg.min_bond {
        None
    } else {
        let w = stake.u128() / (cfg.tokens_per_weight.u128());
        Some(w as u64)
    }
}

pub fn handle_claim(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<HandleResponse, ContractError> {
    let sender_raw = deps.api.canonical_address(&info.sender)?;
    let release = claim_tokens(deps.storage, &sender_raw, &env.block, None)?;
    if release == Uint128(0) {
        return Err(ContractError::NothingToClaim {});
    }

    let config = CONFIG.load(deps.storage)?;
    let amount = coins(release.u128(), config.denom);

    let messages = vec![BankMsg::Send {
        from_address: env.contract.address,
        to_address: info.sender,
        amount,
    }
    .into()];

    Ok(HandleResponse {
        messages,
        attributes: vec![],
        data: None,
    })
}

fn assert_admin(
    api: &dyn Api,
    sender: &HumanAddr,
    admin: Option<CanonicalAddr>,
) -> Result<(), ContractError> {
    let owner = match admin {
        Some(x) => x,
        None => return Err(ContractError::Unauthorized {}),
    };
    if api.canonical_address(sender)? != owner {
        Err(ContractError::Unauthorized {})
    } else {
        Ok(())
    }
}

pub fn handle_add_hook(
    deps: DepsMut,
    info: MessageInfo,
    addr: HumanAddr,
) -> Result<HandleResponse, ContractError> {
    let admin = ADMIN.load(deps.storage)?;
    assert_admin(deps.api, &info.sender, admin)?;
    add_hook(deps.storage, addr)?;
    Ok(HandleResponse::default())
}

pub fn handle_remove_hook(
    deps: DepsMut,
    info: MessageInfo,
    addr: HumanAddr,
) -> Result<HandleResponse, ContractError> {
    let admin = ADMIN.load(deps.storage)?;
    assert_admin(deps.api, &info.sender, admin)?;
    remove_hook(deps.storage, addr)?;
    Ok(HandleResponse::default())
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Member {
            addr,
            at_height: height,
        } => to_binary(&query_member(deps, addr, height)?),
        QueryMsg::ListMembers { start_after, limit } => {
            to_binary(&list_members(deps, start_after, limit)?)
        }
        QueryMsg::Admin {} => to_binary(&query_admin(deps)?),
        QueryMsg::TotalWeight {} => to_binary(&query_total_weight(deps)?),
        QueryMsg::Hooks {} => to_binary(&query_hooks(deps)?),
        QueryMsg::Claims { address } => to_binary(&query_claims(deps, address)?),
        QueryMsg::Staked { address } => to_binary(&query_staked(deps, address)?),
    }
}

fn query_admin(deps: Deps) -> StdResult<AdminResponse> {
    let canon = ADMIN.load(deps.storage)?;
    let admin = canon.map(|c| deps.api.human_address(&c)).transpose()?;
    Ok(AdminResponse { admin })
}

fn query_hooks(deps: Deps) -> StdResult<HooksResponse> {
    let hooks = HOOKS.may_load(deps.storage)?.unwrap_or_default();
    Ok(HooksResponse { hooks })
}

fn query_total_weight(deps: Deps) -> StdResult<TotalWeightResponse> {
    let weight = TOTAL.load(deps.storage)?;
    Ok(TotalWeightResponse { weight })
}

pub fn query_claims(deps: Deps, address: HumanAddr) -> StdResult<ClaimsResponse> {
    let address_raw = deps.api.canonical_address(&address)?;
    let claims = CLAIMS
        .may_load(deps.storage, &address_raw)?
        .unwrap_or_default();
    Ok(ClaimsResponse { claims })
}

pub fn query_staked(deps: Deps, address: HumanAddr) -> StdResult<StakedResponse> {
    let address_raw = deps.api.canonical_address(&address)?;
    let stake = STAKE
        .may_load(deps.storage, &address_raw)?
        .unwrap_or_default();
    let denom = CONFIG.load(deps.storage)?.denom;
    Ok(StakedResponse {
        stake: coin(stake.u128(), denom),
    })
}

fn query_member(deps: Deps, addr: HumanAddr, height: Option<u64>) -> StdResult<MemberResponse> {
    let raw = deps.api.canonical_address(&addr)?;
    let weight = match height {
        Some(h) => MEMBERS.may_load_at_height(deps.storage, &raw, h),
        None => MEMBERS.may_load(deps.storage, &raw),
    }?;
    Ok(MemberResponse { weight })
}

// settings for pagination
const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;

fn list_members(
    deps: Deps,
    start_after: Option<HumanAddr>,
    limit: Option<u32>,
) -> StdResult<MemberListResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let canon = maybe_canonical(deps.api, start_after)?;
    let start = canon.map(Bound::exclusive);

    let api = &deps.api;
    let members: StdResult<Vec<_>> = MEMBERS
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (key, weight) = item?;
            Ok(Member {
                addr: api.human_address(&CanonicalAddr::from(key))?,
                weight,
            })
        })
        .collect();

    Ok(MemberListResponse { members: members? })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{from_slice, OwnedDeps, Querier, Storage};
    // use cw0::hooks::{HOOK_ALREADY_REGISTERED, HOOK_NOT_REGISTERED};
    use cw0::Duration;
    use cw4::{member_key, TOTAL_KEY};

    const ADMIN: &str = "juan";
    const USER1: &str = "somebody";
    const USER2: &str = "else";
    const USER3: &str = "funny";

    const DENOM: &str = "stake";
    const TOKENS_PER_WEIGHT: Uint128 = Uint128(1_000);
    const MIN_BOND: Uint128 = Uint128(5_000);
    const UNBONDING_BLOCKS: u64 = 100;

    fn default_init(deps: DepsMut) {
        do_init(
            deps,
            TOKENS_PER_WEIGHT,
            MIN_BOND,
            Duration::Height(UNBONDING_BLOCKS),
        )
    }

    fn do_init(
        deps: DepsMut,
        tokens_per_weight: Uint128,
        min_bond: Uint128,
        unbonding_period: Duration,
    ) {
        let msg = InitMsg {
            stake: DENOM.to_string(),
            tokens_per_weight,
            min_bond,
            unbonding_period,
            admin: Some(ADMIN.into()),
        };
        let info = mock_info("creator", &[]);
        init(deps, mock_env(), info, msg).unwrap();
    }

    fn bond_stake(mut deps: DepsMut, user1: u128, user2: u128, user3: u128, height_delta: u64) {
        let mut env = mock_env();
        env.block.height += height_delta;

        for (addr, stake) in &[(USER1, user1), (USER2, user2), (USER3, user3)] {
            if *stake != 0 {
                let msg = HandleMsg::Bond {};
                let info = mock_info(HumanAddr::from(*addr), &coins(*stake, DENOM));
                handle(deps.branch(), env.clone(), info, msg).unwrap();
            }
        }
    }

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(&[]);
        default_init(deps.as_mut());

        // it worked, let's query the state
        let res = query_admin(deps.as_ref()).unwrap();
        assert_eq!(Some(HumanAddr::from(ADMIN)), res.admin);

        let res = query_total_weight(deps.as_ref()).unwrap();
        assert_eq!(0, res.weight);
    }

    #[test]
    fn try_update_admin() {
        let mut deps = mock_dependencies(&[]);
        default_init(deps.as_mut());

        // a member cannot update admin
        let err = update_admin(deps.as_mut(), USER1.into(), Some(USER3.into())).unwrap_err();
        match err {
            ContractError::Unauthorized {} => {}
            e => panic!("Unexpected error: {}", e),
        }

        // admin can change it
        update_admin(deps.as_mut(), ADMIN.into(), Some(USER3.into())).unwrap();
        assert_eq!(
            query_admin(deps.as_ref()).unwrap().admin,
            Some(USER3.into())
        );

        // and unset it
        update_admin(deps.as_mut(), USER3.into(), None).unwrap();
        assert_eq!(query_admin(deps.as_ref()).unwrap().admin, None);

        // no one can change it now
        let err = update_admin(deps.as_mut(), USER3.into(), Some(USER1.into())).unwrap_err();
        match err {
            ContractError::Unauthorized {} => {}
            e => panic!("Unexpected error: {}", e),
        }
    }

    // this tests the member queries
    fn assert_users<S: Storage, A: Api, Q: Querier>(
        deps: &OwnedDeps<S, A, Q>,
        user1_weight: Option<u64>,
        user2_weight: Option<u64>,
        user3_weight: Option<u64>,
        height: Option<u64>,
    ) {
        let member1 = query_member(deps.as_ref(), USER1.into(), height).unwrap();
        assert_eq!(member1.weight, user1_weight);

        let member2 = query_member(deps.as_ref(), USER2.into(), height).unwrap();
        assert_eq!(member2.weight, user2_weight);

        let member3 = query_member(deps.as_ref(), USER3.into(), height).unwrap();
        assert_eq!(member3.weight, user3_weight);

        // this is only valid if we are not doing a historical query
        if height.is_none() {
            // compute expected metrics
            let weights = vec![user1_weight, user2_weight, user3_weight];
            let sum: u64 = weights.iter().map(|x| x.unwrap_or_default()).sum();
            let count = weights.iter().filter(|x| x.is_some()).count();

            // TODO: more detailed compare?
            let members = list_members(deps.as_ref(), None, None).unwrap();
            assert_eq!(count, members.members.len());

            let total = query_total_weight(deps.as_ref()).unwrap();
            assert_eq!(sum, total.weight); // 17 - 11 + 15 = 21
        }
    }

    #[test]
    fn bond_stake_adds_membership() {
        let mut deps = mock_dependencies(&[]);
        default_init(deps.as_mut());
        let height = mock_env().block.height;

        // Assert original weights
        assert_users(&deps, None, None, None, None);

        // ensure it rounds down, and respects cut-off
        bond_stake(deps.as_mut(), 12_000, 7_500, 4_000, 1);

        // Assert updated weights
        assert_users(&deps, Some(12), Some(7), None, None);

        // add some more, ensure the sum is properly respected (7.5 + 7.6 = 15 not 14)
        bond_stake(deps.as_mut(), 0, 7_600, 1_200, 2);

        // Assert updated weights
        assert_users(&deps, Some(12), Some(15), Some(5), None);

        // check historical queries all work
        assert_users(&deps, None, None, None, Some(height + 1)); // before first stake
        assert_users(&deps, Some(12), Some(7), None, Some(height + 2)); // after first stake
        assert_users(&deps, Some(12), Some(15), Some(5), Some(height + 3)); // after second stake
    }

    #[test]
    fn raw_queries_work() {
        // add will over-write and remove have no effect
        let mut deps = mock_dependencies(&[]);
        default_init(deps.as_mut());
        // Set values as (11, 6, None)
        bond_stake(deps.as_mut(), 11_000, 6_000, 0, 1);

        // get total from raw key
        let total_raw = deps.storage.get(TOTAL_KEY).unwrap();
        let total: u64 = from_slice(&total_raw).unwrap();
        assert_eq!(17, total);

        // get member votes from raw key
        let member2_canon = deps.api.canonical_address(&USER2.into()).unwrap();
        let member2_raw = deps.storage.get(&member_key(&member2_canon)).unwrap();
        let member2: u64 = from_slice(&member2_raw).unwrap();
        assert_eq!(6, member2);

        // and handle misses
        let member3_canon = deps.api.canonical_address(&USER3.into()).unwrap();
        let member3_raw = deps.storage.get(&member_key(&member3_canon));
        assert_eq!(None, member3_raw);
    }

    // TODO: unbonding -> claims
    // TODO: accepting claims
    // TODO: edge-case -> weight = 0, also min_bond = 0

    // #[test]
    // fn add_remove_hooks() {
    //     // add will over-write and remove have no effect
    //     let mut deps = mock_dependencies(&[]);
    //     default_init(deps.as_mut());
    //
    //     let hooks = query_hooks(deps.as_ref()).unwrap();
    //     assert!(hooks.hooks.is_empty());
    //
    //     let contract1 = HumanAddr::from("hook1");
    //     let contract2 = HumanAddr::from("hook2");
    //
    //     let add_msg = HandleMsg::AddHook {
    //         addr: contract1.clone(),
    //     };
    //
    //     // non-admin cannot add hook
    //     let user_info = mock_info(USER1, &[]);
    //     let err = handle(
    //         deps.as_mut(),
    //         mock_env(),
    //         user_info.clone(),
    //         add_msg.clone(),
    //     )
    //     .unwrap_err();
    //     match err {
    //         ContractError::Unauthorized {} => {}
    //         e => panic!("Unexpected error: {}", e),
    //     }
    //
    //     // admin can add it, and it appears in the query
    //     let admin_info = mock_info(ADMIN, &[]);
    //     let _ = handle(
    //         deps.as_mut(),
    //         mock_env(),
    //         admin_info.clone(),
    //         add_msg.clone(),
    //     )
    //     .unwrap();
    //     let hooks = query_hooks(deps.as_ref()).unwrap();
    //     assert_eq!(hooks.hooks, vec![contract1.clone()]);
    //
    //     // cannot remove a non-registered contract
    //     let remove_msg = HandleMsg::RemoveHook {
    //         addr: contract2.clone(),
    //     };
    //     let err = handle(
    //         deps.as_mut(),
    //         mock_env(),
    //         admin_info.clone(),
    //         remove_msg.clone(),
    //     )
    //     .unwrap_err();
    //
    //     match err {
    //         ContractError::Std(StdError::GenericErr { msg, .. }) => {
    //             assert_eq!(msg, HOOK_NOT_REGISTERED)
    //         }
    //         e => panic!("Unexpected error: {}", e),
    //     }
    //
    //     // add second contract
    //     let add_msg2 = HandleMsg::AddHook {
    //         addr: contract2.clone(),
    //     };
    //     let _ = handle(deps.as_mut(), mock_env(), admin_info.clone(), add_msg2).unwrap();
    //     let hooks = query_hooks(deps.as_ref()).unwrap();
    //     assert_eq!(hooks.hooks, vec![contract1.clone(), contract2.clone()]);
    //
    //     // cannot re-add an existing contract
    //     let err = handle(
    //         deps.as_mut(),
    //         mock_env(),
    //         admin_info.clone(),
    //         add_msg.clone(),
    //     )
    //     .unwrap_err();
    //     match err {
    //         ContractError::Std(StdError::GenericErr { msg, .. }) => {
    //             assert_eq!(msg, HOOK_ALREADY_REGISTERED)
    //         }
    //         e => panic!("Unexpected error: {}", e),
    //     }
    //
    //     // non-admin cannot remove
    //     let remove_msg = HandleMsg::RemoveHook {
    //         addr: contract1.clone(),
    //     };
    //     let err = handle(
    //         deps.as_mut(),
    //         mock_env(),
    //         user_info.clone(),
    //         remove_msg.clone(),
    //     )
    //     .unwrap_err();
    //     match err {
    //         ContractError::Unauthorized {} => {}
    //         e => panic!("Unexpected error: {}", e),
    //     }
    //
    //     // remove the original
    //     let _ = handle(
    //         deps.as_mut(),
    //         mock_env(),
    //         admin_info.clone(),
    //         remove_msg.clone(),
    //     )
    //     .unwrap();
    //     let hooks = query_hooks(deps.as_ref()).unwrap();
    //     assert_eq!(hooks.hooks, vec![contract2.clone()]);
    // }
    //
    // #[test]
    // fn hooks_fire() {
    //     let mut deps = mock_dependencies(&[]);
    //     default_init(deps.as_mut());
    //
    //     let hooks = query_hooks(deps.as_ref()).unwrap();
    //     assert!(hooks.hooks.is_empty());
    //
    //     let contract1 = HumanAddr::from("hook1");
    //     let contract2 = HumanAddr::from("hook2");
    //
    //     // register 2 hooks
    //     let admin_info = mock_info(ADMIN, &[]);
    //     let add_msg = HandleMsg::AddHook {
    //         addr: contract1.clone(),
    //     };
    //     let add_msg2 = HandleMsg::AddHook {
    //         addr: contract2.clone(),
    //     };
    //     for msg in vec![add_msg, add_msg2] {
    //         let _ = handle(deps.as_mut(), mock_env(), admin_info.clone(), msg).unwrap();
    //     }
    //
    //     // make some changes - add 3, remove 2, and update 1
    //     // USER1 is updated and remove in the same call, we should remove this an add member3
    //     let add = vec![
    //         Member {
    //             addr: USER1.into(),
    //             weight: 20,
    //         },
    //         Member {
    //             addr: USER3.into(),
    //             weight: 5,
    //         },
    //     ];
    //     let remove = vec![USER2.into()];
    //     let msg = HandleMsg::UpdateMembers { remove, add };
    //
    //     // admin updates properly
    //     assert_users(&deps, Some(11), Some(6), None, None);
    //     let res = handle(deps.as_mut(), mock_env(), admin_info.clone(), msg).unwrap();
    //     assert_users(&deps, Some(20), None, Some(5), None);
    //
    //     // ensure 2 messages for the 2 hooks
    //     assert_eq!(res.messages.len(), 2);
    //     // same order as in the message (adds first, then remove)
    //     let diffs = vec![
    //         MemberDiff::new(USER1, Some(11), Some(20)),
    //         MemberDiff::new(USER3, None, Some(5)),
    //         MemberDiff::new(USER2, Some(6), None),
    //     ];
    //     let hook_msg = MemberChangedHookMsg { diffs };
    //     let msg1 = hook_msg.clone().into_cosmos_msg(contract1).unwrap();
    //     let msg2 = hook_msg.into_cosmos_msg(contract2).unwrap();
    //     assert_eq!(res.messages, vec![msg1, msg2]);
    // }
}
