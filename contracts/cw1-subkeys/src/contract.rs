use schemars::JsonSchema;
use std::fmt;
use std::ops::{AddAssign, Sub};

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, to_binary, BankMsg, Binary, CanonicalAddr, Coin, CosmosMsg, Deps, DepsMut, Empty, Env,
    HumanAddr, MessageInfo, Order, Response, StakingMsg, StdError, StdResult,
};
use cw0::{maybe_canonical, Expiration};
use cw1::CanExecuteResponse;
use cw1_whitelist::{
    contract::{
        execute_freeze, execute_update_admins, instantiate as whitelist_instantiate,
        query_admin_list,
    },
    msg::InstantiateMsg,
    state::ADMIN_LIST,
};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{
    AllAllowancesResponse, AllPermissionsResponse, AllowanceInfo, ExecuteMsg, PermissionsInfo,
    QueryMsg,
};
use crate::state::{Allowance, Permissions, ALLOWANCES, PERMISSIONS};
use cw_storage_plus::Bound;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cw1-subkeys";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    let result = whitelist_instantiate(deps.branch(), env, info, msg)?;
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(result)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    // Note: implement this function with different type to add support for custom messages
    // and then import the rest of this contract code.
    msg: ExecuteMsg<Empty>,
) -> Result<Response<Empty>, ContractError> {
    match msg {
        ExecuteMsg::Execute { msgs } => execute_execute(deps, env, info, msgs),
        ExecuteMsg::Freeze {} => Ok(execute_freeze(deps, env, info)?),
        ExecuteMsg::UpdateAdmins { admins } => Ok(execute_update_admins(deps, env, info, admins)?),
        ExecuteMsg::IncreaseAllowance {
            spender,
            amount,
            expires,
        } => execute_increase_allowance(deps, env, info, spender, amount, expires),
        ExecuteMsg::DecreaseAllowance {
            spender,
            amount,
            expires,
        } => execute_decrease_allowance(deps, env, info, spender, amount, expires),
        ExecuteMsg::SetPermissions {
            spender,
            permissions,
        } => execute_set_permissions(deps, env, info, spender, permissions),
    }
}

pub fn execute_execute<T>(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msgs: Vec<CosmosMsg<T>>,
) -> Result<Response<T>, ContractError>
where
    T: Clone + fmt::Debug + PartialEq + JsonSchema,
{
    let cfg = ADMIN_LIST.load(deps.storage)?;
    let owner_raw = &deps.api.canonical_address(&info.sender)?;
    // this is the admin behavior (same as cw1-whitelist)
    if cfg.is_admin(owner_raw) {
        let mut res = Response::default();
        res.messages = msgs;
        res.attributes = vec![attr("action", "execute"), attr("owner", info.sender)];
        Ok(res)
    } else {
        for msg in &msgs {
            match msg {
                CosmosMsg::Staking(staking_msg) => {
                    let perm = PERMISSIONS.may_load(deps.storage, &owner_raw)?;
                    let perm = perm.ok_or(ContractError::NotAllowed {})?;
                    check_staking_permissions(staking_msg, perm)?;
                }
                CosmosMsg::Bank(BankMsg::Send {
                    to_address: _,
                    amount,
                }) => {
                    let allow = ALLOWANCES.may_load(deps.storage, &owner_raw)?;
                    let mut allowance = allow.ok_or(ContractError::NoAllowance {})?;
                    // Decrease allowance
                    allowance.balance = allowance.balance.sub(amount.clone())?;
                    ALLOWANCES.save(deps.storage, &owner_raw, &allowance)?;
                }
                _ => {
                    return Err(ContractError::MessageTypeRejected {});
                }
            }
        }
        // Relay messages
        let res = Response {
            submessages: vec![],
            messages: msgs,
            attributes: vec![attr("action", "execute"), attr("owner", info.sender)],
            data: None,
        };
        Ok(res)
    }
}

pub fn check_staking_permissions(
    staking_msg: &StakingMsg,
    permissions: Permissions,
) -> Result<bool, ContractError> {
    match staking_msg {
        StakingMsg::Delegate { .. } => {
            if !permissions.delegate {
                return Err(ContractError::DelegatePerm {});
            }
        }
        StakingMsg::Undelegate { .. } => {
            if !permissions.undelegate {
                return Err(ContractError::UnDelegatePerm {});
            }
        }
        StakingMsg::Redelegate { .. } => {
            if !permissions.redelegate {
                return Err(ContractError::ReDelegatePerm {});
            }
        }
        StakingMsg::Withdraw { .. } => {
            if !permissions.withdraw {
                return Err(ContractError::WithdrawPerm {});
            }
        }
        s => panic!("Unsupported staking message: {:?}", s),
    }
    Ok(true)
}

pub fn execute_increase_allowance<T>(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    spender: HumanAddr,
    amount: Coin,
    expires: Option<Expiration>,
) -> Result<Response<T>, ContractError>
where
    T: Clone + fmt::Debug + PartialEq + JsonSchema,
{
    let cfg = ADMIN_LIST.load(deps.storage)?;
    let spender_raw = &deps.api.canonical_address(&spender)?;
    let owner_raw = &deps.api.canonical_address(&info.sender)?;

    if !cfg.is_admin(&owner_raw) {
        return Err(ContractError::Unauthorized {});
    }
    if spender_raw == owner_raw {
        return Err(ContractError::CannotSetOwnAccount {});
    }

    ALLOWANCES.update::<_, StdError>(deps.storage, &spender_raw, |allow| {
        let mut allowance = allow.unwrap_or_default();
        if let Some(exp) = expires {
            allowance.expires = exp;
        }
        allowance.balance.add_assign(amount.clone());
        Ok(allowance)
    })?;

    let res = Response {
        submessages: vec![],
        messages: vec![],
        attributes: vec![
            attr("action", "increase_allowance"),
            attr("owner", info.sender),
            attr("spender", spender),
            attr("denomination", amount.denom),
            attr("amount", amount.amount),
        ],
        data: None,
    };
    Ok(res)
}

pub fn execute_decrease_allowance<T>(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    spender: HumanAddr,
    amount: Coin,
    expires: Option<Expiration>,
) -> Result<Response<T>, ContractError>
where
    T: Clone + fmt::Debug + PartialEq + JsonSchema,
{
    let cfg = ADMIN_LIST.load(deps.storage)?;
    let spender_raw = &deps.api.canonical_address(&spender)?;
    let owner_raw = &deps.api.canonical_address(&info.sender)?;

    if !cfg.is_admin(&owner_raw) {
        return Err(ContractError::Unauthorized {});
    }
    if spender_raw == owner_raw {
        return Err(ContractError::CannotSetOwnAccount {});
    }

    let allowance = ALLOWANCES.update::<_, ContractError>(deps.storage, &spender_raw, |allow| {
        // Fail fast
        let mut allowance = allow.ok_or(ContractError::NoAllowance {})?;
        if let Some(exp) = expires {
            allowance.expires = exp;
        }
        allowance.balance = allowance.balance.sub_saturating(amount.clone())?; // Tolerates underflows (amount bigger than balance), but fails if there are no tokens at all for the denom (report potential errors)
        Ok(allowance)
    })?;
    if allowance.balance.is_empty() {
        ALLOWANCES.remove(deps.storage, &spender_raw);
    }

    let res = Response {
        submessages: vec![],
        messages: vec![],
        attributes: vec![
            attr("action", "decrease_allowance"),
            attr("owner", deps.api.human_address(owner_raw)?),
            attr("spender", spender),
            attr("denomination", amount.denom),
            attr("amount", amount.amount),
        ],
        data: None,
    };
    Ok(res)
}

pub fn execute_set_permissions<T>(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    spender: HumanAddr,
    perm: Permissions,
) -> Result<Response<T>, ContractError>
where
    T: Clone + fmt::Debug + PartialEq + JsonSchema,
{
    let cfg = ADMIN_LIST.load(deps.storage)?;
    let spender_raw = &deps.api.canonical_address(&spender)?;
    let owner_raw = &deps.api.canonical_address(&info.sender)?;

    if !cfg.is_admin(&owner_raw) {
        return Err(ContractError::Unauthorized {});
    }
    if spender_raw == owner_raw {
        return Err(ContractError::CannotSetOwnAccount {});
    }
    PERMISSIONS.save(deps.storage, &spender_raw, &perm)?;

    let res = Response {
        submessages: vec![],
        messages: vec![],
        attributes: vec![
            attr("action", "set_permissions"),
            attr("owner", deps.api.human_address(owner_raw)?),
            attr("spender", spender),
            attr("permissions", perm),
        ],
        data: None,
    };
    Ok(res)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::AdminList {} => to_binary(&query_admin_list(deps)?),
        QueryMsg::Allowance { spender } => to_binary(&query_allowance(deps, spender)?),
        QueryMsg::Permissions { spender } => to_binary(&query_permissions(deps, spender)?),
        QueryMsg::CanExecute { sender, msg } => to_binary(&query_can_execute(deps, sender, msg)?),
        QueryMsg::AllAllowances { start_after, limit } => {
            to_binary(&query_all_allowances(deps, start_after, limit)?)
        }
        QueryMsg::AllPermissions { start_after, limit } => {
            to_binary(&query_all_permissions(deps, start_after, limit)?)
        }
    }
}

// if the subkey has no allowance, return an empty struct (not an error)
pub fn query_allowance(deps: Deps, spender: HumanAddr) -> StdResult<Allowance> {
    let subkey = deps.api.canonical_address(&spender)?;
    let allow = ALLOWANCES
        .may_load(deps.storage, &subkey)?
        .unwrap_or_default();
    Ok(allow)
}

// if the subkey has no permissions, return an empty struct (not an error)
pub fn query_permissions(deps: Deps, spender: HumanAddr) -> StdResult<Permissions> {
    let subkey = deps.api.canonical_address(&spender)?;
    let permissions = PERMISSIONS
        .may_load(deps.storage, &subkey)?
        .unwrap_or_default();
    Ok(permissions)
}

fn query_can_execute(
    deps: Deps,
    sender: HumanAddr,
    msg: CosmosMsg,
) -> StdResult<CanExecuteResponse> {
    Ok(CanExecuteResponse {
        can_execute: can_execute(deps, sender, msg)?,
    })
}

// this can just return booleans and the query_can_execute wrapper creates the struct once, not on every path
fn can_execute(deps: Deps, sender: HumanAddr, msg: CosmosMsg) -> StdResult<bool> {
    let owner_raw = deps.api.canonical_address(&sender)?;
    let cfg = ADMIN_LIST.load(deps.storage)?;
    if cfg.is_admin(&owner_raw) {
        return Ok(true);
    }
    match msg {
        CosmosMsg::Bank(BankMsg::Send { amount, .. }) => {
            // now we check if there is enough allowance for this message
            let allowance = ALLOWANCES.may_load(deps.storage, &owner_raw)?;
            match allowance {
                // if there is an allowance, we subtract the requested amount to ensure it is covered (error on underflow)
                Some(allow) => Ok(allow.balance.sub(amount).is_ok()),
                None => Ok(false),
            }
        }
        CosmosMsg::Staking(staking_msg) => {
            let perm_opt = PERMISSIONS.may_load(deps.storage, &owner_raw)?;
            match perm_opt {
                Some(permission) => Ok(check_staking_permissions(&staking_msg, permission).is_ok()),
                None => Ok(false),
            }
        }
        _ => Ok(false),
    }
}

const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;

fn calc_limit(request: Option<u32>) -> usize {
    request.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize
}

// return a list of all allowances here
pub fn query_all_allowances(
    deps: Deps,
    start_after: Option<HumanAddr>,
    limit: Option<u32>,
) -> StdResult<AllAllowancesResponse> {
    let limit = calc_limit(limit);
    let canon = maybe_canonical(deps.api, start_after)?;
    let start = canon.map(Bound::exclusive);

    let api = &deps.api;
    let res: StdResult<Vec<AllowanceInfo>> = ALLOWANCES
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            item.and_then(|(k, allow)| {
                Ok(AllowanceInfo {
                    spender: api.human_address(&CanonicalAddr::from(k))?,
                    balance: allow.balance,
                    expires: allow.expires,
                })
            })
        })
        .collect();
    Ok(AllAllowancesResponse { allowances: res? })
}

// return a list of all permissions here
pub fn query_all_permissions(
    deps: Deps,
    start_after: Option<HumanAddr>,
    limit: Option<u32>,
) -> StdResult<AllPermissionsResponse> {
    let limit = calc_limit(limit);
    let canon = maybe_canonical(deps.api, start_after)?;
    let start = canon.map(Bound::exclusive);

    let api = &deps.api;
    let res: StdResult<Vec<PermissionsInfo>> = PERMISSIONS
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            item.and_then(|(k, perm)| {
                Ok(PermissionsInfo {
                    spender: api.human_address(&CanonicalAddr::from(k))?,
                    permissions: perm,
                })
            })
        })
        .collect();
    Ok(AllPermissionsResponse { permissions: res? })
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coin, coins, Api, StakingMsg};

    use cw0::NativeBalance;
    use cw1_whitelist::msg::AdminListResponse;
    use cw2::{get_contract_version, ContractVersion};

    use crate::state::Permissions;

    use super::*;

    // this will set up instantiation for other tests
    fn setup_test_case(
        mut deps: DepsMut,
        info: &MessageInfo,
        admins: &[HumanAddr],
        spenders: &[HumanAddr],
        allowances: &[Coin],
        expirations: &[Expiration],
    ) {
        // Instantiate a contract with admins
        let instantiate_msg = InstantiateMsg {
            admins: admins.to_vec(),
            mutable: true,
        };
        instantiate(deps.branch(), mock_env(), info.clone(), instantiate_msg).unwrap();

        // Add subkeys with initial allowances
        for (spender, expiration) in spenders.iter().zip(expirations) {
            for amount in allowances {
                let msg = ExecuteMsg::IncreaseAllowance {
                    spender: spender.clone(),
                    amount: amount.clone(),
                    expires: Some(expiration.clone()),
                };
                execute(deps.branch(), mock_env(), info.clone(), msg).unwrap();
            }
        }
    }

    #[test]
    fn get_contract_version_works() {
        let mut deps = mock_dependencies(&[]);

        let owner = HumanAddr::from("admin0001");
        let admins = vec![owner.clone(), HumanAddr::from("admin0002")];

        let spender1 = HumanAddr::from("spender0001");
        let spender2 = HumanAddr::from("spender0002");
        let initial_spenders = vec![spender1.clone(), spender2.clone()];

        // Same allowances for all spenders, for simplicity
        let denom1 = "token1";
        let amount1 = 1111;

        let allow1 = coin(amount1, denom1);
        let initial_allowances = vec![allow1.clone()];

        let expires_never = Expiration::Never {};
        let initial_expirations = vec![expires_never.clone(), expires_never.clone()];

        let info = mock_info(owner, &[]);
        setup_test_case(
            deps.as_mut(),
            &info,
            &admins,
            &initial_spenders,
            &initial_allowances,
            &initial_expirations,
        );

        assert_eq!(
            ContractVersion {
                contract: CONTRACT_NAME.to_string(),
                version: CONTRACT_VERSION.to_string(),
            },
            get_contract_version(&mut deps.storage).unwrap()
        )
    }

    #[test]
    fn query_allowance_works() {
        let mut deps = mock_dependencies(&[]);

        let owner = HumanAddr::from("admin0001");
        let admins = vec![owner.clone(), HumanAddr::from("admin0002")];

        let spender1 = HumanAddr::from("spender0001");
        let spender2 = HumanAddr::from("spender0002");
        let spender3 = HumanAddr::from("spender0003");
        let initial_spenders = vec![spender1.clone(), spender2.clone()];

        // Same allowances for all spenders, for simplicity
        let denom1 = "token1";
        let amount1 = 1111;

        let allow1 = coin(amount1, denom1);
        let initial_allowances = vec![allow1.clone()];

        let expires_never = Expiration::Never {};
        let initial_expirations = vec![expires_never.clone(), expires_never.clone()];

        let info = mock_info(owner, &[]);
        setup_test_case(
            deps.as_mut(),
            &info,
            &admins,
            &initial_spenders,
            &initial_allowances,
            &initial_expirations,
        );

        // Check allowances work for accounts with balances
        let allowance = query_allowance(deps.as_ref(), spender1.clone()).unwrap();
        assert_eq!(
            allowance,
            Allowance {
                balance: NativeBalance(vec![allow1.clone()]),
                expires: expires_never.clone(),
            }
        );
        let allowance = query_allowance(deps.as_ref(), spender2.clone()).unwrap();
        assert_eq!(
            allowance,
            Allowance {
                balance: NativeBalance(vec![allow1.clone()]),
                expires: expires_never.clone(),
            }
        );

        // Check allowances work for accounts with no balance
        let allowance = query_allowance(deps.as_ref(), spender3.clone()).unwrap();
        assert_eq!(allowance, Allowance::default(),);
    }

    #[test]
    fn query_all_allowances_works() {
        let mut deps = mock_dependencies(&[]);

        let owner = HumanAddr::from("admin0001");
        let admins = vec![owner.clone(), HumanAddr::from("admin0002")];

        let spender1 = HumanAddr::from("spender0001");
        let spender2 = HumanAddr::from("spender0002");
        let spender3 = HumanAddr::from("spender0003");
        let initial_spenders = vec![spender1.clone(), spender2.clone(), spender3.clone()];

        // Same allowances for all spenders, for simplicity
        let initial_allowances = coins(1234, "mytoken");
        let expires_later = Expiration::AtHeight(12345);
        let initial_expirations = vec![
            Expiration::Never {},
            Expiration::Never {},
            expires_later.clone(),
        ];

        let info = mock_info(owner, &[]);
        setup_test_case(
            deps.as_mut(),
            &info,
            &admins,
            &initial_spenders,
            &initial_allowances,
            &initial_expirations,
        );

        // let's try pagination
        let allowances = query_all_allowances(deps.as_ref(), None, Some(2))
            .unwrap()
            .allowances;
        assert_eq!(2, allowances.len());
        assert_eq!(
            allowances[0],
            AllowanceInfo {
                spender: spender2,
                balance: NativeBalance(initial_allowances.clone()),
                expires: Expiration::Never {},
            }
        );
        assert_eq!(
            allowances[1],
            AllowanceInfo {
                spender: spender3.clone(),
                balance: NativeBalance(initial_allowances.clone()),
                expires: expires_later,
            }
        );

        // now continue from after the last one
        let allowances = query_all_allowances(deps.as_ref(), Some(spender3), Some(2))
            .unwrap()
            .allowances;
        assert_eq!(1, allowances.len());
        assert_eq!(
            allowances[0],
            AllowanceInfo {
                spender: spender1,
                balance: NativeBalance(initial_allowances.clone()),
                expires: Expiration::Never {},
            }
        );
    }

    #[test]
    fn query_permissions_works() {
        let mut deps = mock_dependencies(&[]);

        let owner = HumanAddr::from("admin0001");
        let admins = vec![owner.clone()];

        // spender1 has every permission to stake
        let spender1 = HumanAddr::from("spender0001");
        // spender2 do not have permission
        let spender2 = HumanAddr::from("spender0002");
        // non existent spender
        let spender3 = HumanAddr::from("spender0003");

        let god_mode = Permissions {
            delegate: true,
            redelegate: true,
            undelegate: true,
            withdraw: true,
        };

        let info = mock_info(owner.clone(), &[]);
        // Instantiate a contract with admins
        let instantiate_msg = InstantiateMsg {
            admins: admins.clone(),
            mutable: true,
        };
        instantiate(deps.as_mut(), mock_env(), info.clone(), instantiate_msg).unwrap();

        let setup_perm_msg1 = ExecuteMsg::SetPermissions {
            spender: spender1.clone(),
            permissions: god_mode,
        };
        execute(deps.as_mut(), mock_env(), info.clone(), setup_perm_msg1).unwrap();

        let setup_perm_msg2 = ExecuteMsg::SetPermissions {
            spender: spender2.clone(),
            // default is no permission
            permissions: Default::default(),
        };
        execute(deps.as_mut(), mock_env(), info, setup_perm_msg2).unwrap();

        let permissions = query_permissions(deps.as_ref(), spender1.clone()).unwrap();
        assert_eq!(permissions, god_mode);

        let permissions = query_permissions(deps.as_ref(), spender2.clone()).unwrap();
        assert_eq!(
            permissions,
            Permissions {
                delegate: false,
                redelegate: false,
                undelegate: false,
                withdraw: false,
            },
        );

        // no permission is set. should return false
        let permissions = query_permissions(deps.as_ref(), spender3.clone()).unwrap();
        assert_eq!(
            permissions,
            Permissions {
                delegate: false,
                redelegate: false,
                undelegate: false,
                withdraw: false,
            },
        );

        //
    }

    #[test]
    fn query_all_permissions_works() {
        let mut deps = mock_dependencies(&[]);

        let owner = HumanAddr::from("admin0001");
        let admins = vec![owner.clone(), HumanAddr::from("admin0002")];

        let spender1 = HumanAddr::from("spender0001");
        let spender2 = HumanAddr::from("spender0002");
        let spender3 = HumanAddr::from("spender0003");

        let god_mode = Permissions {
            delegate: true,
            redelegate: true,
            undelegate: true,
            withdraw: true,
        };

        let noob_mode = Permissions {
            delegate: false,
            redelegate: false,
            undelegate: false,
            withdraw: false,
        };

        let info = mock_info(owner, &[]);

        // Instantiate a contract with admins
        let instantiate_msg = InstantiateMsg {
            admins: admins.clone(),
            mutable: true,
        };
        instantiate(deps.as_mut(), mock_env(), info.clone(), instantiate_msg).unwrap();

        let setup_perm_msg1 = ExecuteMsg::SetPermissions {
            spender: spender1.clone(),
            permissions: god_mode,
        };
        execute(deps.as_mut(), mock_env(), info.clone(), setup_perm_msg1).unwrap();

        let setup_perm_msg2 = ExecuteMsg::SetPermissions {
            spender: spender2.clone(),
            permissions: noob_mode,
        };
        execute(deps.as_mut(), mock_env(), info.clone(), setup_perm_msg2).unwrap();

        let setup_perm_msg3 = ExecuteMsg::SetPermissions {
            spender: spender3.clone(),
            permissions: noob_mode,
        };
        execute(deps.as_mut(), mock_env(), info, setup_perm_msg3).unwrap();

        // let's try pagination
        let permissions = query_all_permissions(deps.as_ref(), None, Some(2))
            .unwrap()
            .permissions;
        assert_eq!(2, permissions.len());
        assert_eq!(
            permissions[0],
            PermissionsInfo {
                spender: spender2,
                permissions: noob_mode,
            }
        );
        assert_eq!(
            permissions[1],
            PermissionsInfo {
                spender: spender3.clone(),
                permissions: noob_mode,
            }
        );

        // now continue from after the last one
        let permissions = query_all_permissions(deps.as_ref(), Some(spender3), Some(2))
            .unwrap()
            .permissions;
        assert_eq!(1, permissions.len());
        assert_eq!(
            permissions[0],
            PermissionsInfo {
                spender: spender1,
                permissions: god_mode,
            }
        );
    }

    #[test]
    fn update_admins_and_query() {
        let mut deps = mock_dependencies(&[]);

        let owner = HumanAddr::from("admin0001");
        let admin2 = HumanAddr::from("admin0002");
        let admin3 = HumanAddr::from("admin0003");
        let initial_admins = vec![owner.clone(), admin2.clone()];

        let info = mock_info(owner.clone(), &[]);
        setup_test_case(
            deps.as_mut(),
            &info,
            &initial_admins,
            &vec![],
            &vec![],
            &vec![],
        );

        // Verify
        let config = query_admin_list(deps.as_ref()).unwrap();
        assert_eq!(
            config,
            AdminListResponse {
                admins: initial_admins.clone(),
                mutable: true,
            }
        );

        // Add a third (new) admin
        let new_admins = vec![owner.clone(), admin2.clone(), admin3.clone()];
        let msg = ExecuteMsg::UpdateAdmins {
            admins: new_admins.clone(),
        };
        execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        // Verify
        let config = query_admin_list(deps.as_ref()).unwrap();
        println!("config: {:#?}", config);
        assert_eq!(
            config,
            AdminListResponse {
                admins: new_admins,
                mutable: true,
            }
        );

        // Set admin3 as the only admin
        let msg = ExecuteMsg::UpdateAdmins {
            admins: vec![admin3.clone()],
        };
        execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        // Verify admin3 is now the sole admin
        let config = query_admin_list(deps.as_ref()).unwrap();
        println!("config: {:#?}", config);
        assert_eq!(
            config,
            AdminListResponse {
                admins: vec![admin3.clone()],
                mutable: true,
            }
        );

        // Try to add owner back
        let msg = ExecuteMsg::UpdateAdmins {
            admins: vec![admin3.clone(), owner.clone()],
        };
        let res = execute(deps.as_mut(), mock_env(), info, msg);

        // Verify it fails (admin3 is now the owner)
        assert!(res.is_err());

        // Connect as admin3
        let info = mock_info(admin3.clone(), &[]);
        // Add owner back
        let msg = ExecuteMsg::UpdateAdmins {
            admins: vec![admin3.clone(), owner.clone()],
        };
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Verify
        let config = query_admin_list(deps.as_ref()).unwrap();
        println!("config: {:#?}", config);
        assert_eq!(
            config,
            AdminListResponse {
                admins: vec![admin3, owner],
                mutable: true,
            }
        );
    }

    #[test]
    fn increase_allowances() {
        let mut deps = mock_dependencies(&[]);

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

        let info = mock_info(owner, &[]);
        setup_test_case(
            deps.as_mut(),
            &info,
            &admins,
            &initial_spenders,
            &initial_allowances,
            &initial_expirations,
        );

        // Add to spender1 account (expires = None) => don't change Expiration
        let msg = ExecuteMsg::IncreaseAllowance {
            spender: spender1.clone(),
            amount: allow1.clone(),
            expires: None,
        };
        execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        // Verify
        let allowance = query_allowance(deps.as_ref(), spender1.clone()).unwrap();
        assert_eq!(
            allowance,
            Allowance {
                balance: NativeBalance(vec![coin(amount1 * 2, &allow1.denom), allow2.clone()]),
                expires: expires_height.clone(),
            }
        );

        // Add to spender2 account (expires = Some)
        let msg = ExecuteMsg::IncreaseAllowance {
            spender: spender2.clone(),
            amount: allow3.clone(),
            expires: Some(expires_height.clone()),
        };
        execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        // Verify
        let allowance = query_allowance(deps.as_ref(), spender2.clone()).unwrap();
        assert_eq!(
            allowance,
            Allowance {
                balance: NativeBalance(vec![allow1.clone(), allow2.clone(), allow3.clone()]),
                expires: expires_height.clone(),
            }
        );

        // Add to spender3 (new account) (expires = None) => default Expiration::Never
        let msg = ExecuteMsg::IncreaseAllowance {
            spender: spender3.clone(),
            amount: allow1.clone(),
            expires: None,
        };
        execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        // Verify
        let allowance = query_allowance(deps.as_ref(), spender3.clone()).unwrap();
        assert_eq!(
            allowance,
            Allowance {
                balance: NativeBalance(vec![allow1.clone()]),
                expires: expires_never.clone(),
            }
        );

        // Add to spender4 (new account) (expires = Some)
        let msg = ExecuteMsg::IncreaseAllowance {
            spender: spender4.clone(),
            amount: allow2.clone(),
            expires: Some(expires_time.clone()),
        };
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Verify
        let allowance = query_allowance(deps.as_ref(), spender4.clone()).unwrap();
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
        let mut deps = mock_dependencies(&[]);

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

        let info = mock_info(owner, &[]);
        setup_test_case(
            deps.as_mut(),
            &info,
            &admins,
            &initial_spenders,
            &initial_allowances,
            &initial_expirations,
        );

        // Subtract from spender1 (existing) account (has none of that denom)
        let msg = ExecuteMsg::DecreaseAllowance {
            spender: spender1.clone(),
            amount: allow3.clone(),
            expires: None,
        };
        let res = execute(deps.as_mut(), mock_env(), info.clone(), msg);

        // Verify
        assert!(res.is_err());
        // Verify everything stays the same for that spender
        let allowance = query_allowance(deps.as_ref(), spender1.clone()).unwrap();
        assert_eq!(
            allowance,
            Allowance {
                balance: NativeBalance(vec![allow1.clone(), allow2.clone()]),
                expires: expires_height.clone(),
            }
        );

        // Subtract from spender2 (existing) account (brings denom to 0, other denoms left)
        let msg = ExecuteMsg::DecreaseAllowance {
            spender: spender2.clone(),
            amount: allow2.clone(),
            expires: None,
        };
        execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        // Verify
        let allowance = query_allowance(deps.as_ref(), spender2.clone()).unwrap();
        assert_eq!(
            allowance,
            Allowance {
                balance: NativeBalance(vec![allow1.clone()]),
                expires: expires_never.clone(),
            }
        );

        // Subtract from spender1 (existing) account (brings denom to > 0)
        let msg = ExecuteMsg::DecreaseAllowance {
            spender: spender1.clone(),
            amount: coin(amount1 / 2, denom1),
            expires: None,
        };
        execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        // Verify
        let allowance = query_allowance(deps.as_ref(), spender1.clone()).unwrap();
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
        let msg = ExecuteMsg::DecreaseAllowance {
            spender: spender2.clone(),
            amount: allow1.clone(),
            expires: None,
        };
        execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        // Verify
        let allowance = query_allowance(deps.as_ref(), spender2.clone()).unwrap();
        assert_eq!(allowance, Allowance::default());

        // Subtract from spender2 (empty) account (should error)
        let msg = ExecuteMsg::DecreaseAllowance {
            spender: spender2.clone(),
            amount: allow1.clone(),
            expires: None,
        };
        let res = execute(deps.as_mut(), mock_env(), info.clone(), msg);

        // Verify
        assert!(res.is_err());

        // Subtract from spender1 (existing) account (underflows denom => should delete denom)
        let msg = ExecuteMsg::DecreaseAllowance {
            spender: spender1.clone(),
            amount: coin(amount1 * 10, denom1),
            expires: None,
        };
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Verify
        let allowance = query_allowance(deps.as_ref(), spender1.clone()).unwrap();
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
        let mut deps = mock_dependencies(&[]);

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

        let info = mock_info(owner.clone(), &[]);
        setup_test_case(
            deps.as_mut(),
            &info,
            &admins,
            &initial_spenders,
            &initial_allowances,
            &initial_expirations,
        );

        // Create Send message
        let msgs = vec![BankMsg::Send {
            to_address: spender2.clone(),
            amount: coins(1000, "token1"),
        }
        .into()];

        let execute_msg = ExecuteMsg::Execute { msgs: msgs.clone() };

        // spender2 cannot spend funds (no initial allowance)
        let info = mock_info(&spender2, &[]);
        let res = execute(deps.as_mut(), mock_env(), info, execute_msg.clone());
        assert_eq!(ContractError::NoAllowance {}, res.unwrap_err());

        // But spender1 can (he has enough funds)
        let info = mock_info(&spender1, &[]);
        let res = execute(deps.as_mut(), mock_env(), info.clone(), execute_msg.clone()).unwrap();
        assert_eq!(res.messages, msgs);
        assert_eq!(
            res.attributes,
            vec![attr("action", "execute"), attr("owner", spender1.clone())]
        );

        // And then cannot (not enough funds anymore)
        let res = execute(deps.as_mut(), mock_env(), info, execute_msg.clone());
        match res.unwrap_err() {
            ContractError::Std(StdError::Underflow { .. }) => {}
            e => panic!("unexpected error: {:?}", e),
        }

        // Owner / admins can do anything (at the contract level)
        let info = mock_info(&owner.clone(), &[]);
        let res = execute(deps.as_mut(), mock_env(), info, execute_msg.clone()).unwrap();
        assert_eq!(res.messages, msgs);
        assert_eq!(
            res.attributes,
            vec![attr("action", "execute"), attr("owner", owner.clone())]
        );

        // For admins, even other message types are allowed
        let other_msgs = vec![CosmosMsg::Custom(Empty {})];
        let execute_msg = ExecuteMsg::Execute {
            msgs: other_msgs.clone(),
        };

        let info = mock_info(&owner, &[]);
        let res = execute(deps.as_mut(), mock_env(), info, execute_msg.clone()).unwrap();
        assert_eq!(res.messages, other_msgs);
        assert_eq!(
            res.attributes,
            vec![attr("action", "execute"), attr("owner", owner)]
        );

        // But not for mere mortals
        let info = mock_info(&spender1, &[]);
        let res = execute(deps.as_mut(), mock_env(), info, execute_msg.clone());
        assert_eq!(ContractError::MessageTypeRejected {}, res.unwrap_err());
    }

    #[test]
    fn staking_permission_checks() {
        let mut deps = mock_dependencies(&[]);

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

        let info = mock_info(owner.clone(), &[]);
        // Instantiate a contract with admins
        let instantiate_msg = InstantiateMsg {
            admins: admins.clone(),
            mutable: true,
        };
        instantiate(deps.as_mut(), mock_env(), info.clone(), instantiate_msg).unwrap();

        let setup_perm_msg1 = ExecuteMsg::SetPermissions {
            spender: spender1.clone(),
            permissions: god_mode,
        };
        execute(deps.as_mut(), mock_env(), info.clone(), setup_perm_msg1).unwrap();

        let setup_perm_msg2 = ExecuteMsg::SetPermissions {
            spender: spender2.clone(),
            // default is no permission
            permissions: Default::default(),
        };
        // default is no permission
        execute(deps.as_mut(), mock_env(), info.clone(), setup_perm_msg2).unwrap();

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
            let info = mock_info(&spender1, &[]);
            let res = execute(
                deps.as_mut(),
                mock_env(),
                info,
                ExecuteMsg::Execute { msgs: msg.clone() },
            );
            assert!(res.is_ok())
        }

        // spender2 cannot execute (no permission)
        for msg in &msgs {
            let info = mock_info(&spender2, &[]);
            let res = execute(
                deps.as_mut(),
                mock_env(),
                info.clone(),
                ExecuteMsg::Execute { msgs: msg.clone() },
            );
            assert!(res.is_err())
        }

        // test mixed permissions
        let spender3 = HumanAddr::from("spender0003");
        let setup_perm_msg3 = ExecuteMsg::SetPermissions {
            spender: spender3.clone(),
            permissions: Permissions {
                delegate: false,
                redelegate: true,
                undelegate: true,
                withdraw: false,
            },
        };
        execute(deps.as_mut(), mock_env(), info, setup_perm_msg3).unwrap();
        let info = mock_info(&spender3, &[]);
        let res = execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            ExecuteMsg::Execute {
                msgs: msg_delegate.clone(),
            },
        );
        // FIXME need better error check here
        assert!(res.is_err());
        let res = execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            ExecuteMsg::Execute {
                msgs: msg_redelegate.clone(),
            },
        );
        assert!(res.is_ok());
        let res = execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            ExecuteMsg::Execute {
                msgs: msg_undelegate.clone(),
            },
        );
        assert!(res.is_ok());
        let res = execute(
            deps.as_mut(),
            mock_env(),
            info,
            ExecuteMsg::Execute {
                msgs: msg_withdraw.clone(),
            },
        );
        assert!(res.is_err())
    }

    // tests permissions and allowances are independent features and does not affect each other
    #[test]
    fn permissions_allowances_independent() {
        let mut deps = mock_dependencies(&[]);

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

        let info = mock_info(owner.clone(), &[]);
        // Instantiate a contract with admins
        let instantiate_msg = InstantiateMsg {
            admins: admins.clone(),
            mutable: true,
        };
        instantiate(deps.as_mut(), mock_env(), info.clone(), instantiate_msg).unwrap();

        // setup permission and then allowance and check if changed
        let setup_perm_msg = ExecuteMsg::SetPermissions {
            spender: spender1.clone(),
            permissions: perm,
        };
        execute(deps.as_mut(), mock_env(), info.clone(), setup_perm_msg).unwrap();

        let setup_allowance_msg = ExecuteMsg::IncreaseAllowance {
            spender: spender1.clone(),
            amount: coin.clone(),
            expires: None,
        };
        execute(deps.as_mut(), mock_env(), info.clone(), setup_allowance_msg).unwrap();

        let res_perm = query_permissions(deps.as_ref(), spender1.clone()).unwrap();
        assert_eq!(perm, res_perm);
        let res_allow = query_allowance(deps.as_ref(), spender1.clone()).unwrap();
        assert_eq!(allow, res_allow);

        // setup allowance and then permission and check if changed
        let setup_allowance_msg = ExecuteMsg::IncreaseAllowance {
            spender: spender2.clone(),
            amount: coin.clone(),
            expires: None,
        };
        execute(deps.as_mut(), mock_env(), info.clone(), setup_allowance_msg).unwrap();

        let setup_perm_msg = ExecuteMsg::SetPermissions {
            spender: spender2.clone(),
            permissions: perm,
        };
        execute(deps.as_mut(), mock_env(), info, setup_perm_msg).unwrap();

        let res_perm = query_permissions(deps.as_ref(), spender2.clone()).unwrap();
        assert_eq!(perm, res_perm);
        let res_allow = query_allowance(deps.as_ref(), spender2.clone()).unwrap();
        assert_eq!(allow, res_allow);
    }

    #[test]
    fn can_execute_query_works() {
        let mut deps = mock_dependencies(&[]);

        let owner = HumanAddr::from("admin007");
        let spender = HumanAddr::from("spender808");
        let anyone = HumanAddr::from("anyone");

        let info = mock_info(owner.clone(), &[]);
        // spender has allowance of 55000 ushell
        setup_test_case(
            deps.as_mut(),
            &info,
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
        let _ = PERMISSIONS.save(&mut deps.storage, &spender_raw, &perm);

        // let us make some queries... different msg types by owner and by other
        let send_msg = CosmosMsg::Bank(BankMsg::Send {
            to_address: anyone.clone(),
            amount: coins(12345, "ushell"),
        });
        let send_msg_large = CosmosMsg::Bank(BankMsg::Send {
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
        let res = query_can_execute(deps.as_ref(), owner.clone(), send_msg.clone()).unwrap();
        assert_eq!(res.can_execute, true);
        let res = query_can_execute(deps.as_ref(), owner.clone(), send_msg_large.clone()).unwrap();
        assert_eq!(res.can_execute, true);
        // owner can stake
        let res =
            query_can_execute(deps.as_ref(), owner.clone(), staking_delegate_msg.clone()).unwrap();
        assert_eq!(res.can_execute, true);

        // spender can send small
        let res = query_can_execute(deps.as_ref(), spender.clone(), send_msg.clone()).unwrap();
        assert_eq!(res.can_execute, true);
        // not too big
        let res =
            query_can_execute(deps.as_ref(), spender.clone(), send_msg_large.clone()).unwrap();
        assert_eq!(res.can_execute, false);
        // spender can send staking msgs if permissioned
        let res = query_can_execute(deps.as_ref(), spender.clone(), staking_delegate_msg.clone())
            .unwrap();
        assert_eq!(res.can_execute, true);
        let res = query_can_execute(deps.as_ref(), spender.clone(), staking_withdraw_msg.clone())
            .unwrap();
        assert_eq!(res.can_execute, false);

        // random person cannot do anything
        let res = query_can_execute(deps.as_ref(), anyone.clone(), send_msg.clone()).unwrap();
        assert_eq!(res.can_execute, false);
        let res = query_can_execute(deps.as_ref(), anyone.clone(), send_msg_large.clone()).unwrap();
        assert_eq!(res.can_execute, false);
        let res =
            query_can_execute(deps.as_ref(), anyone.clone(), staking_delegate_msg.clone()).unwrap();
        assert_eq!(res.can_execute, false);
        let res =
            query_can_execute(deps.as_ref(), anyone.clone(), staking_withdraw_msg.clone()).unwrap();
        assert_eq!(res.can_execute, false);
    }
}
