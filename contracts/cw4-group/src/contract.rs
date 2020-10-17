use cosmwasm_std::{
    to_binary, Api, Binary, CanonicalAddr, Env, Extern, HandleResponse, HumanAddr, InitResponse,
    MessageInfo, Order, Querier, StdResult, Storage,
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
pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    _env: Env,
    _info: MessageInfo,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    set_contract_version(&mut deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    create(deps, msg.admin, msg.members)?;
    Ok(InitResponse::default())
}

// create is the init logic with set_contract_version removed so it can more
// easily be imported in other contracts
pub fn create<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    admin: Option<HumanAddr>,
    members: Vec<Member>,
) -> StdResult<()> {
    let admin_raw = maybe_canonical(deps.api, admin)?;
    ADMIN.save(&mut deps.storage, &admin_raw)?;

    let mut total = 0u64;
    for member in members.into_iter() {
        total += member.weight;
        let raw = deps.api.canonical_address(&member.addr)?;
        MEMBERS.save(&mut deps.storage, &raw, &member.weight)?;
    }
    TOTAL.save(&mut deps.storage, &total)?;

    Ok(())
}

// And declare a custom Error variant for the ones where you will want to make use of it
pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    _env: Env,
    info: MessageInfo,
    msg: HandleMsg,
) -> Result<HandleResponse, ContractError> {
    match msg {
        HandleMsg::UpdateAdmin { admin } => handle_update_admin(deps, info, admin),
        HandleMsg::UpdateMembers { add, remove } => handle_update_members(deps, info, add, remove),
    }
}

pub fn handle_update_admin<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    info: MessageInfo,
    new_admin: Option<HumanAddr>,
) -> Result<HandleResponse, ContractError> {
    update_admin(deps, info.sender, new_admin)?;
    Ok(HandleResponse::default())
}

// the logic from handle_update_admin extracted for easier import
pub fn update_admin<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    sender: HumanAddr,
    new_admin: Option<HumanAddr>,
) -> Result<Option<CanonicalAddr>, ContractError> {
    let api = deps.api;
    ADMIN.update(&mut deps.storage, |state| -> Result<_, ContractError> {
        assert_admin(api, sender, state)?;
        let new_admin = maybe_canonical(api, new_admin)?;
        Ok(new_admin)
    })
}

pub fn handle_update_members<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    info: MessageInfo,
    add: Vec<Member>,
    remove: Vec<HumanAddr>,
) -> Result<HandleResponse, ContractError> {
    update_members(deps, info.sender, add, remove)?;
    Ok(HandleResponse::default())
}

// the logic from handle_update_admin extracted for easier import
pub fn update_members<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    sender: HumanAddr,
    to_add: Vec<Member>,
    to_remove: Vec<HumanAddr>,
) -> Result<(), ContractError> {
    let admin = ADMIN.load(&deps.storage)?;
    assert_admin(deps.api, sender, admin)?;

    let mut total = TOTAL.load(&deps.storage)?;

    // add all new members and update total
    for add in to_add.into_iter() {
        let raw = deps.api.canonical_address(&add.addr)?;
        MEMBERS.update(&mut deps.storage, &raw, |old| -> StdResult<_> {
            total -= old.unwrap_or_default();
            total += add.weight;
            Ok(add.weight)
        })?;
    }

    for remove in to_remove.into_iter() {
        let raw = deps.api.canonical_address(&remove)?;
        total -= MEMBERS.may_load(&deps.storage, &raw)?.unwrap_or_default();
        MEMBERS.remove(&mut deps.storage, &raw);
    }

    Ok(())
}

fn assert_admin<A: Api>(
    api: A,
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

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    _env: Env,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::Member { addr } => to_binary(&query_member(deps, addr)?),
        QueryMsg::ListMembers { start_after, limit } => {
            to_binary(&list_members(deps, start_after, limit)?)
        }
        QueryMsg::Admin {} => to_binary(&query_admin(deps)?),
        QueryMsg::TotalWeight {} => to_binary(&query_total_weight(deps)?),
    }
}

fn query_admin<S: Storage, A: Api, Q: Querier>(deps: &Extern<S, A, Q>) -> StdResult<AdminResponse> {
    let canon = ADMIN.load(&deps.storage)?;
    let admin = canon.map(|c| deps.api.human_address(&c)).transpose()?;
    Ok(AdminResponse { admin })
}

fn query_total_weight<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> StdResult<TotalWeightResponse> {
    let weight = TOTAL.load(&deps.storage)?;
    Ok(TotalWeightResponse { weight })
}

fn query_member<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    addr: HumanAddr,
) -> StdResult<MemberResponse> {
    let raw = deps.api.canonical_address(&addr)?;
    let weight = MEMBERS.may_load(&deps.storage, &raw)?;
    Ok(MemberResponse { weight })
}

// settings for pagination
const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;

fn list_members<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    start_after: Option<HumanAddr>,
    limit: Option<u32>,
) -> StdResult<MemberListResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let canon = maybe_canonical(deps.api, start_after)?;
    let start = canon.map(Bound::exclusive);

    let api = &deps.api;
    let members: StdResult<Vec<_>> = MEMBERS
        .range(&deps.storage, start, None, Order::Ascending)
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
    use cosmwasm_std::{coins, from_binary};

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(&[]);

        let msg = InitMsg { count: 17 };
        let info = mock_info("creator", &coins(1000, "earth"));

        // we can just call .unwrap() to assert this was a success
        let res = init(&mut deps, mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // it worked, let's query the state
        let res = query(&deps, mock_env(), QueryMsg::GetCount {}).unwrap();
        let value: CountResponse = from_binary(&res).unwrap();
        assert_eq!(17, value.count);
    }

    #[test]
    fn increment() {
        let mut deps = mock_dependencies(&coins(2, "token"));

        let msg = InitMsg { count: 17 };
        let info = mock_info("creator", &coins(2, "token"));
        let _res = init(&mut deps, mock_env(), info, msg).unwrap();

        // beneficiary can release it
        let info = mock_info("anyone", &coins(2, "token"));
        let msg = HandleMsg::Increment {};
        let _res = handle(&mut deps, mock_env(), info, msg).unwrap();

        // should increase counter by 1
        let res = query(&deps, mock_env(), QueryMsg::GetCount {}).unwrap();
        let value: CountResponse = from_binary(&res).unwrap();
        assert_eq!(18, value.count);
    }

    #[test]
    fn reset() {
        let mut deps = mock_dependencies(&coins(2, "token"));

        let msg = InitMsg { count: 17 };
        let info = mock_info("creator", &coins(2, "token"));
        let _res = init(&mut deps, mock_env(), info, msg).unwrap();

        // beneficiary can release it
        let unauth_info = mock_info("anyone", &coins(2, "token"));
        let msg = HandleMsg::Reset { count: 5 };
        let res = handle(&mut deps, mock_env(), unauth_info, msg);
        match res {
            Err(ContractError::Unauthorized {}) => {}
            _ => panic!("Must return unauthorized error"),
        }

        // only the original creator can reset the counter
        let auth_info = mock_info("creator", &coins(2, "token"));
        let msg = HandleMsg::Reset { count: 5 };
        let _res = handle(&mut deps, mock_env(), auth_info, msg).unwrap();

        // should now be 5
        let res = query(&deps, mock_env(), QueryMsg::GetCount {}).unwrap();
        let value: CountResponse = from_binary(&res).unwrap();
        assert_eq!(5, value.count);
    }
}
