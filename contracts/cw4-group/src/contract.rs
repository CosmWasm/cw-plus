use cosmwasm_std::{
    to_binary, Api, Binary, CanonicalAddr, Context, Deps, DepsMut, Env, HandleResponse, HumanAddr,
    InitResponse, MessageInfo, Order, StdResult,
};
use cw0::maybe_canonical;
use cw2::set_contract_version;
use cw4::{
    AdminResponse, HooksResponse, Member, MemberChangedHookMsg, MemberDiff, MemberListResponse,
    MemberResponse, TotalWeightResponse,
};
use cw_storage_plus::Bound;

use crate::error::ContractError;
use crate::msg::{HandleMsg, InitMsg, QueryMsg};
use crate::state::{ADMIN, HOOKS, MEMBERS, TOTAL};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cw4-group";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

// Note, you can use StdResult in some functions where you do not
// make use of the custom errors
pub fn init(deps: DepsMut, env: Env, _info: MessageInfo, msg: InitMsg) -> StdResult<InitResponse> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    create(deps, msg.admin, msg.members, env.block.height)?;
    Ok(InitResponse::default())
}

// create is the init logic with set_contract_version removed so it can more
// easily be imported in other contracts
pub fn create(
    deps: DepsMut,
    admin: Option<HumanAddr>,
    members: Vec<Member>,
    height: u64,
) -> StdResult<()> {
    let admin_raw = maybe_canonical(deps.api, admin)?;
    ADMIN.save(deps.storage, &admin_raw)?;

    let mut total = 0u64;
    for member in members.into_iter() {
        total += member.weight;
        let raw = deps.api.canonical_address(&member.addr)?;
        MEMBERS.save(deps.storage, &raw, &member.weight, height)?;
    }
    TOTAL.save(deps.storage, &total)?;

    Ok(())
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
        HandleMsg::UpdateMembers { add, remove } => {
            handle_update_members(deps, env, info, add, remove)
        }
        HandleMsg::AddHook { addr } => handle_add_hook(deps, info, addr),
        HandleMsg::RemoveHook { addr } => handle_remove_hook(deps, info, addr),
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
        assert_admin(api, sender, state)?;
        let new_admin = maybe_canonical(api, new_admin)?;
        Ok(new_admin)
    })
}

pub fn handle_update_members(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    add: Vec<Member>,
    remove: Vec<HumanAddr>,
) -> Result<HandleResponse, ContractError> {
    // make the local update
    let diff = update_members(deps.branch(), env.block.height, info.sender, add, remove)?;
    // call all registered hooks
    let mut ctx = Context::new();
    for h in HOOKS.may_load(deps.storage)?.unwrap_or_default() {
        let msg = diff.clone().into_cosmos_msg(h)?;
        ctx.add_message(msg);
    }
    Ok(ctx.into())
}

// the logic from handle_update_admin extracted for easier import
pub fn update_members(
    deps: DepsMut,
    height: u64,
    sender: HumanAddr,
    to_add: Vec<Member>,
    to_remove: Vec<HumanAddr>,
) -> Result<MemberChangedHookMsg, ContractError> {
    let admin = ADMIN.load(deps.storage)?;
    assert_admin(deps.api, sender, admin)?;

    let mut total = TOTAL.load(deps.storage)?;
    let mut diffs: Vec<MemberDiff> = vec![];

    // add all new members and update total
    for add in to_add.into_iter() {
        let raw = deps.api.canonical_address(&add.addr)?;
        MEMBERS.update(deps.storage, &raw, height, |old| -> StdResult<_> {
            total -= old.unwrap_or_default();
            total += add.weight;
            diffs.push(MemberDiff::new(add.addr, old, Some(add.weight)));
            Ok(add.weight)
        })?;
    }

    for remove in to_remove.into_iter() {
        let raw = deps.api.canonical_address(&remove)?;
        let old = MEMBERS.may_load(deps.storage, &raw)?;
        // Only process this if they were actually in the list before
        if let Some(weight) = old {
            diffs.push(MemberDiff::new(remove, Some(weight), None));
            total -= weight;
            MEMBERS.remove(deps.storage, &raw, height)?;
        }
    }

    TOTAL.save(deps.storage, &total)?;
    Ok(MemberChangedHookMsg { diffs })
}

fn assert_admin(
    api: &dyn Api,
    sender: HumanAddr,
    admin: Option<CanonicalAddr>,
) -> Result<(), ContractError> {
    let owner = match admin {
        Some(x) => x,
        None => return Err(ContractError::Unauthorized {}),
    };
    if api.canonical_address(&sender)? != owner {
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
    assert_admin(deps.api, info.sender, admin)?;

    let mut hooks = HOOKS.may_load(deps.storage)?.unwrap_or_default();
    if !hooks.iter().any(|h| h == &addr) {
        hooks.push(addr);
    } else {
        return Err(ContractError::HookAlreadyRegistered {});
    }
    HOOKS.save(deps.storage, &hooks)?;
    Ok(HandleResponse::default())
}

pub fn handle_remove_hook(
    deps: DepsMut,
    info: MessageInfo,
    addr: HumanAddr,
) -> Result<HandleResponse, ContractError> {
    let admin = ADMIN.load(deps.storage)?;
    assert_admin(deps.api, info.sender, admin)?;

    let mut hooks = HOOKS.load(deps.storage)?;
    if let Some(p) = hooks.iter().position(|x| x == &addr) {
        hooks.remove(p);
    } else {
        return Err(ContractError::HookNotRegistered {});
    }
    HOOKS.save(deps.storage, &hooks)?;
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
    use cw4::{member_key, TOTAL_KEY};

    const ADMIN: &str = "juan";
    const USER1: &str = "somebody";
    const USER2: &str = "else";
    const USER3: &str = "funny";

    fn do_init(deps: DepsMut) {
        let msg = InitMsg {
            admin: Some(ADMIN.into()),
            members: vec![
                Member {
                    addr: USER1.into(),
                    weight: 11,
                },
                Member {
                    addr: USER2.into(),
                    weight: 6,
                },
            ],
        };
        let info = mock_info("creator", &[]);
        init(deps, mock_env(), info, msg).unwrap();
    }

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(&[]);
        do_init(deps.as_mut());

        // it worked, let's query the state
        let res = query_admin(deps.as_ref()).unwrap();
        assert_eq!(Some(HumanAddr::from(ADMIN)), res.admin);

        let res = query_total_weight(deps.as_ref()).unwrap();
        assert_eq!(17, res.weight);
    }

    #[test]
    fn try_update_admin() {
        let mut deps = mock_dependencies(&[]);
        do_init(deps.as_mut());

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

    #[test]
    fn try_member_queries() {
        let mut deps = mock_dependencies(&[]);
        do_init(deps.as_mut());

        let member1 = query_member(deps.as_ref(), USER1.into(), None).unwrap();
        assert_eq!(member1.weight, Some(11));

        let member2 = query_member(deps.as_ref(), USER2.into(), None).unwrap();
        assert_eq!(member2.weight, Some(6));

        let member3 = query_member(deps.as_ref(), USER3.into(), None).unwrap();
        assert_eq!(member3.weight, None);

        let members = list_members(deps.as_ref(), None, None).unwrap();
        assert_eq!(members.members.len(), 2);
        // TODO: assert the set is proper
    }

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
    fn add_new_remove_old_member() {
        let mut deps = mock_dependencies(&[]);
        do_init(deps.as_mut());

        // add a new one and remove existing one
        let add = vec![Member {
            addr: USER3.into(),
            weight: 15,
        }];
        let remove = vec![USER1.into()];

        // non-admin cannot update
        let height = mock_env().block.height;
        let err = update_members(
            deps.as_mut(),
            height + 5,
            USER1.into(),
            add.clone(),
            remove.clone(),
        )
        .unwrap_err();
        match err {
            ContractError::Unauthorized {} => {}
            e => panic!("Unexpected error: {}", e),
        }

        // Test the values from init
        assert_users(&deps, Some(11), Some(6), None, None);
        // Note all values were set at height, the beginning of that block was all None
        assert_users(&deps, None, None, None, Some(height));
        // This will get us the values at the start of the block after init (expected initial values)
        assert_users(&deps, Some(11), Some(6), None, Some(height + 1));

        // admin updates properly
        update_members(deps.as_mut(), height + 10, ADMIN.into(), add, remove).unwrap();

        // updated properly
        assert_users(&deps, None, Some(6), Some(15), None);

        // snapshot still shows old value
        assert_users(&deps, Some(11), Some(6), None, Some(height + 1));
    }

    #[test]
    fn add_old_remove_new_member() {
        // add will over-write and remove have no effect
        let mut deps = mock_dependencies(&[]);
        do_init(deps.as_mut());

        // add a new one and remove existing one
        let add = vec![Member {
            addr: USER1.into(),
            weight: 4,
        }];
        let remove = vec![USER3.into()];

        // admin updates properly
        let height = mock_env().block.height;
        update_members(deps.as_mut(), height, ADMIN.into(), add, remove).unwrap();
        assert_users(&deps, Some(4), Some(6), None, None);
    }

    #[test]
    fn add_and_remove_same_member() {
        // add will over-write and remove have no effect
        let mut deps = mock_dependencies(&[]);
        do_init(deps.as_mut());

        // USER1 is updated and remove in the same call, we should remove this an add member3
        let add = vec![
            Member {
                addr: USER1.into(),
                weight: 20,
            },
            Member {
                addr: USER3.into(),
                weight: 5,
            },
        ];
        let remove = vec![USER1.into()];

        // admin updates properly
        let height = mock_env().block.height;
        update_members(deps.as_mut(), height, ADMIN.into(), add, remove).unwrap();
        assert_users(&deps, None, Some(6), Some(5), None);
    }

    #[test]
    fn add_remove_hooks() {
        // add will over-write and remove have no effect
        let mut deps = mock_dependencies(&[]);
        do_init(deps.as_mut());

        let hooks = query_hooks(deps.as_ref()).unwrap();
        assert!(hooks.hooks.is_empty());

        let contract1 = HumanAddr::from("hook1");
        let contract2 = HumanAddr::from("hook2");

        let add_msg = HandleMsg::AddHook {
            addr: contract1.clone(),
        };

        // non-admin cannot add hook
        let user_info = mock_info(USER1, &[]);
        let err = handle(
            deps.as_mut(),
            mock_env(),
            user_info.clone(),
            add_msg.clone(),
        )
        .unwrap_err();
        match err {
            ContractError::Unauthorized {} => {}
            e => panic!("Unexpected error: {}", e),
        }

        // admin can add it, and it appears in the query
        let admin_info = mock_info(ADMIN, &[]);
        let _ = handle(
            deps.as_mut(),
            mock_env(),
            admin_info.clone(),
            add_msg.clone(),
        )
        .unwrap();
        let hooks = query_hooks(deps.as_ref()).unwrap();
        assert_eq!(hooks.hooks, vec![contract1.clone()]);

        // cannot remove a non-registered contract
        let remove_msg = HandleMsg::RemoveHook {
            addr: contract2.clone(),
        };
        let err = handle(
            deps.as_mut(),
            mock_env(),
            admin_info.clone(),
            remove_msg.clone(),
        )
        .unwrap_err();
        match err {
            ContractError::HookNotRegistered {} => {}
            e => panic!("Unexpected error: {}", e),
        }

        // add second contract
        let add_msg2 = HandleMsg::AddHook {
            addr: contract2.clone(),
        };
        let _ = handle(deps.as_mut(), mock_env(), admin_info.clone(), add_msg2).unwrap();
        let hooks = query_hooks(deps.as_ref()).unwrap();
        assert_eq!(hooks.hooks, vec![contract1.clone(), contract2.clone()]);

        // cannot re-add an existing contract
        let err = handle(
            deps.as_mut(),
            mock_env(),
            admin_info.clone(),
            add_msg.clone(),
        )
        .unwrap_err();
        match err {
            ContractError::HookAlreadyRegistered {} => {}
            e => panic!("Unexpected error: {}", e),
        }

        // non-admin cannot remove
        let remove_msg = HandleMsg::RemoveHook {
            addr: contract1.clone(),
        };
        let err = handle(
            deps.as_mut(),
            mock_env(),
            user_info.clone(),
            remove_msg.clone(),
        )
        .unwrap_err();
        match err {
            ContractError::Unauthorized {} => {}
            e => panic!("Unexpected error: {}", e),
        }

        // remove the original
        let _ = handle(
            deps.as_mut(),
            mock_env(),
            admin_info.clone(),
            remove_msg.clone(),
        )
        .unwrap();
        let hooks = query_hooks(deps.as_ref()).unwrap();
        assert_eq!(hooks.hooks, vec![contract2.clone()]);
    }

    #[test]
    fn hooks_fire() {
        let mut deps = mock_dependencies(&[]);
        do_init(deps.as_mut());

        let hooks = query_hooks(deps.as_ref()).unwrap();
        assert!(hooks.hooks.is_empty());

        let contract1 = HumanAddr::from("hook1");
        let contract2 = HumanAddr::from("hook2");

        // register 2 hooks
        let admin_info = mock_info(ADMIN, &[]);
        let add_msg = HandleMsg::AddHook {
            addr: contract1.clone(),
        };
        let add_msg2 = HandleMsg::AddHook {
            addr: contract2.clone(),
        };
        for msg in vec![add_msg, add_msg2] {
            let _ = handle(deps.as_mut(), mock_env(), admin_info.clone(), msg).unwrap();
        }

        // make some changes - add 3, remove 2, and update 1
        // USER1 is updated and remove in the same call, we should remove this an add member3
        let add = vec![
            Member {
                addr: USER1.into(),
                weight: 20,
            },
            Member {
                addr: USER3.into(),
                weight: 5,
            },
        ];
        let remove = vec![USER2.into()];
        let msg = HandleMsg::UpdateMembers { remove, add };

        // admin updates properly
        assert_users(&deps, Some(11), Some(6), None, None);
        let res = handle(deps.as_mut(), mock_env(), admin_info.clone(), msg).unwrap();
        assert_users(&deps, Some(20), None, Some(5), None);

        // ensure 2 messages for the 2 hooks
        assert_eq!(res.messages.len(), 2);
        // same order as in the message (adds first, then remove)
        let diffs = vec![
            MemberDiff::new(USER1, Some(11), Some(20)),
            MemberDiff::new(USER3, None, Some(5)),
            MemberDiff::new(USER2, Some(6), None),
        ];
        let hook_msg = MemberChangedHookMsg { diffs };
        let msg1 = hook_msg.clone().into_cosmos_msg(contract1).unwrap();
        let msg2 = hook_msg.into_cosmos_msg(contract2).unwrap();
        assert_eq!(res.messages, vec![msg1, msg2]);
    }

    #[test]
    fn raw_queries_work() {
        // add will over-write and remove have no effect
        let mut deps = mock_dependencies(&[]);
        do_init(deps.as_mut());

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
}
