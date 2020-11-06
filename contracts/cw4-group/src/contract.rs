use cosmwasm_std::{
    to_binary, Api, Binary, CanonicalAddr, Deps, DepsMut, Env, HandleResponse, HumanAddr,
    InitResponse, MessageInfo, Order, StdResult,
};
use cw0::maybe_canonical;
use cw2::set_contract_version;
use cw4::{AdminResponse, Member, MemberListResponse, MemberResponse, TotalWeightResponse};
use cw_storage_plus::Bound;

use crate::error::ContractError;
use crate::msg::{HandleMsg, InitMsg, QueryMsg};
use crate::state::{ADMIN, MEMBERS, TOTAL};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cw4-group";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

// Note, you can use StdResult in some functions where you do not
// make use of the custom errors
pub fn init(deps: DepsMut, _env: Env, _info: MessageInfo, msg: InitMsg) -> StdResult<InitResponse> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    create(deps, msg.admin, msg.members)?;
    Ok(InitResponse::default())
}

// create is the init logic with set_contract_version removed so it can more
// easily be imported in other contracts
pub fn create(deps: DepsMut, admin: Option<HumanAddr>, members: Vec<Member>) -> StdResult<()> {
    let admin_raw = maybe_canonical(deps.api, admin)?;
    ADMIN.save(deps.storage, &admin_raw)?;

    let mut total = 0u64;
    for member in members.into_iter() {
        total += member.weight;
        let raw = deps.api.canonical_address(&member.addr)?;
        MEMBERS.save(deps.storage, &raw, &member.weight)?;
    }
    TOTAL.save(deps.storage, &total)?;

    Ok(())
}

// And declare a custom Error variant for the ones where you will want to make use of it
pub fn handle(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: HandleMsg,
) -> Result<HandleResponse, ContractError> {
    match msg {
        HandleMsg::UpdateAdmin { admin } => handle_update_admin(deps, info, admin),
        HandleMsg::UpdateMembers { add, remove } => handle_update_members(deps, info, add, remove),
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
    deps: DepsMut,
    info: MessageInfo,
    add: Vec<Member>,
    remove: Vec<HumanAddr>,
) -> Result<HandleResponse, ContractError> {
    update_members(deps, info.sender, add, remove)?;
    Ok(HandleResponse::default())
}

// the logic from handle_update_admin extracted for easier import
pub fn update_members(
    deps: DepsMut,
    sender: HumanAddr,
    to_add: Vec<Member>,
    to_remove: Vec<HumanAddr>,
) -> Result<(), ContractError> {
    let admin = ADMIN.load(deps.storage)?;
    assert_admin(deps.api, sender, admin)?;

    let mut total = TOTAL.load(deps.storage)?;

    // add all new members and update total
    for add in to_add.into_iter() {
        let raw = deps.api.canonical_address(&add.addr)?;
        MEMBERS.update(deps.storage, &raw, |old| -> StdResult<_> {
            total -= old.unwrap_or_default();
            total += add.weight;
            Ok(add.weight)
        })?;
    }

    for remove in to_remove.into_iter() {
        let raw = deps.api.canonical_address(&remove)?;
        total -= MEMBERS.may_load(deps.storage, &raw)?.unwrap_or_default();
        MEMBERS.remove(deps.storage, &raw);
    }

    TOTAL.save(deps.storage, &total)?;

    Ok(())
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

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Member { addr } => to_binary(&query_member(deps, addr)?),
        QueryMsg::ListMembers { start_after, limit } => {
            to_binary(&list_members(deps, start_after, limit)?)
        }
        QueryMsg::Admin {} => to_binary(&query_admin(deps)?),
        QueryMsg::TotalWeight {} => to_binary(&query_total_weight(deps)?),
    }
}

fn query_admin(deps: Deps) -> StdResult<AdminResponse> {
    let canon = ADMIN.load(deps.storage)?;
    let admin = canon.map(|c| deps.api.human_address(&c)).transpose()?;
    Ok(AdminResponse { admin })
}

fn query_total_weight(deps: Deps) -> StdResult<TotalWeightResponse> {
    let weight = TOTAL.load(deps.storage)?;
    Ok(TotalWeightResponse { weight })
}

fn query_member(deps: Deps, addr: HumanAddr) -> StdResult<MemberResponse> {
    let raw = deps.api.canonical_address(&addr)?;
    let weight = MEMBERS.may_load(deps.storage, &raw)?;
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

        let member1 = query_member(deps.as_ref(), USER1.into()).unwrap();
        assert_eq!(member1.weight, Some(11));

        let member2 = query_member(deps.as_ref(), USER2.into()).unwrap();
        assert_eq!(member2.weight, Some(6));

        let member3 = query_member(deps.as_ref(), USER3.into()).unwrap();
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
    ) {
        let member1 = query_member(deps.as_ref(), USER1.into()).unwrap();
        assert_eq!(member1.weight, user1_weight);

        let member2 = query_member(deps.as_ref(), USER2.into()).unwrap();
        assert_eq!(member2.weight, user2_weight);

        let member3 = query_member(deps.as_ref(), USER3.into()).unwrap();
        assert_eq!(member3.weight, user3_weight);

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
        let err =
            update_members(deps.as_mut(), USER1.into(), add.clone(), remove.clone()).unwrap_err();
        match err {
            ContractError::Unauthorized {} => {}
            e => panic!("Unexpected error: {}", e),
        }

        // admin updates properly
        update_members(deps.as_mut(), ADMIN.into(), add, remove).unwrap();
        assert_users(&deps, None, Some(6), Some(15));
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
        update_members(deps.as_mut(), ADMIN.into(), add, remove).unwrap();
        assert_users(&deps, Some(4), Some(6), None);
    }

    #[test]
    fn add_and_remove_same_member() {
        // add will over-write and remove have no effect
        let mut deps = mock_dependencies(&[]);
        do_init(deps.as_mut());

        // USER1 is updated and remove in the same line, we should remove this an add member3
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
        update_members(deps.as_mut(), ADMIN.into(), add, remove).unwrap();
        assert_users(&deps, None, Some(6), Some(5));
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
