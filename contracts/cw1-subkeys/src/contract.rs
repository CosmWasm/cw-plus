use schemars::JsonSchema;
use std::fmt;

use cosmwasm_std::{log, to_binary, Api, BankMsg, Binary, CanonicalAddr, Coin, CosmosMsg, Empty, Env, Extern, HandleResponse,
                   HumanAddr, InitResponse, Order, Querier, StdError, StdResult, Storage, StakingMsg};
use cw0::Expiration;
use cw1::CanSendResponse;
use cw1_whitelist::{
    contract::{handle_freeze, handle_update_admins, init as whitelist_init, query_admin_list},
    msg::InitMsg,
    state::admin_list_read,
};
use cw2::set_contract_version;

use crate::msg::{AllAllowancesResponse, AllowanceInfo, HandleMsg, QueryMsg};
use crate::state::{allowances, allowances_read, permissions, Allowance, Permissions, PermissionErr};
use std::ops::{AddAssign, Sub};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cw1-subkeys";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    let result = whitelist_init(deps, env, msg)?;
    set_contract_version(&mut deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(result)
}

pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    // Note: implement this function with different type to add support for custom messages
    // and then import the rest of this contract code.
    msg: HandleMsg<Empty>,
) -> StdResult<HandleResponse<Empty>> {
    match msg {
        HandleMsg::Execute { msgs } => handle_execute(deps, env, msgs),
        HandleMsg::Freeze {} => handle_freeze(deps, env),
        HandleMsg::UpdateAdmins { admins } => handle_update_admins(deps, env, admins),
        HandleMsg::IncreaseAllowance {
            spender,
            amount,
            expires,
        } => handle_increase_allowance(deps, env, spender, amount, expires),
        HandleMsg::DecreaseAllowance {
            spender,
            amount,
            expires,
        } => handle_decrease_allowance(deps, env, spender, amount, expires),
        HandleMsg::SetPermissions {
            spender,
            permissions,
        } => handle_set_permissions(deps, env, spender, permissions),
    }
}

pub fn handle_execute<S: Storage, A: Api, Q: Querier, T>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msgs: Vec<CosmosMsg<T>>,
) -> StdResult<HandleResponse<T>>
where
    T: Clone + fmt::Debug + PartialEq + JsonSchema,
{
    let cfg = admin_list_read(&deps.storage).load()?;
    let owner_raw = &deps.api.canonical_address(&env.message.sender)?;
    // this is the admin behavior (same as cw1-whitelist)
    if cfg.is_admin(owner_raw) {
        let mut res = HandleResponse::default();
        res.messages = msgs;
        res.log = vec![log("action", "execute"), log("owner", env.message.sender)];
        Ok(res)
    } else {
        for msg in &msgs {
            match msg {
                CosmosMsg::Staking(staking_msg) => {
                    let permissions = permissions(&mut deps.storage);
                    let perm = permissions.may_load(owner_raw.as_slice())?;
                    let perm =
                        perm.ok_or_else(|| StdError::not_found("No permissions for this account"))?;

                    check_staking_msg(staking_msg, perm)?;
                }
                CosmosMsg::Bank(BankMsg::Send {
                    from_address: _,
                    to_address: _,
                    amount,
                }) => {
                    let mut allowances = allowances(&mut deps.storage);
                    let allow = allowances.may_load(owner_raw.as_slice())?;
                    let mut allowance =
                        allow.ok_or_else(|| StdError::not_found("No allowance for this account"))?;
                    // Decrease allowance
                    allowance.balance = allowance.balance.sub(amount.clone())?;
                    allowances.save(owner_raw.as_slice(), &allowance)?;
                }
                _ => {
                    return Err(StdError::generic_err("Message type rejected"));
                }
            }
        }
        // Relay messages
        let res = HandleResponse {
            messages: msgs,
            log: vec![log("action", "execute"), log("owner", env.message.sender)],
            data: None,
        };
        Ok(res)
    }
}

pub fn check_staking_msg(staking_msg: &StakingMsg, permissions: Permissions) -> StdResult<()> {
    match staking_msg {
        StakingMsg::Delegate { validator: _, amount: _ } => {
            if !permissions.delegate {
                return Err(StdError::generic_err(PermissionErr::Delegate {}));
            }
        }
        StakingMsg::Undelegate { validator: _, amount: _ } => {
            if !permissions.undelegate {
                return Err(StdError::generic_err(PermissionErr::Undelegate {}));
            }
        }
        StakingMsg::Redelegate { src_validator: _, dst_validator: _, amount: _ } => {
            if !permissions.redelegate {
                return Err(StdError::generic_err(PermissionErr::Redelegate {}));
            }
        }
        StakingMsg::Withdraw { validator: _, recipient: _ } => {
            if !permissions.withdraw {
                return Err(StdError::generic_err(PermissionErr::Withdraw {}));
            }
        }
    }
    return Ok(())
}

pub fn handle_increase_allowance<S: Storage, A: Api, Q: Querier, T>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    spender: HumanAddr,
    amount: Coin,
    expires: Option<Expiration>,
) -> StdResult<HandleResponse<T>>
where
    T: Clone + fmt::Debug + PartialEq + JsonSchema,
{
    let cfg = admin_list_read(&deps.storage).load()?;
    let spender_raw = &deps.api.canonical_address(&spender)?;
    let owner_raw = &deps.api.canonical_address(&env.message.sender)?;

    if !cfg.is_admin(&owner_raw) {
        return Err(StdError::unauthorized());
    }
    if spender_raw == owner_raw {
        return Err(StdError::generic_err("Cannot set allowance to own account"));
    }

    allowances(&mut deps.storage).update(spender_raw.as_slice(), |allow| {
        let mut allowance = allow.unwrap_or_default();
        if let Some(exp) = expires {
            allowance.expires = exp;
        }
        allowance.balance.add_assign(amount.clone());
        Ok(allowance)
    })?;

    let res = HandleResponse {
        messages: vec![],
        log: vec![
            log("action", "increase_allowance"),
            log("owner", env.message.sender),
            log("spender", spender),
            log("denomination", amount.denom),
            log("amount", amount.amount),
        ],
        data: None,
    };
    Ok(res)
}

pub fn handle_decrease_allowance<S: Storage, A: Api, Q: Querier, T>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    spender: HumanAddr,
    amount: Coin,
    expires: Option<Expiration>,
) -> StdResult<HandleResponse<T>>
where
    T: Clone + fmt::Debug + PartialEq + JsonSchema,
{
    let cfg = admin_list_read(&deps.storage).load()?;
    let spender_raw = &deps.api.canonical_address(&spender)?;
    let owner_raw = &deps.api.canonical_address(&env.message.sender)?;

    if !cfg.is_admin(&owner_raw) {
        return Err(StdError::unauthorized());
    }
    if spender_raw == owner_raw {
        return Err(StdError::generic_err("Cannot set allowance to own account"));
    }

    let allowance = allowances(&mut deps.storage).update(spender_raw.as_slice(), |allow| {
        // Fail fast
        let mut allowance =
            allow.ok_or_else(|| StdError::not_found("No allowance for this account"))?;
        if let Some(exp) = expires {
            allowance.expires = exp;
        }
        allowance.balance = allowance.balance.sub_saturating(amount.clone())?; // Tolerates underflows (amount bigger than balance), but fails if there are no tokens at all for the denom (report potential errors)
        Ok(allowance)
    })?;
    if allowance.balance.is_empty() {
        allowances(&mut deps.storage).remove(spender_raw.as_slice());
    }

    let res = HandleResponse {
        messages: vec![],
        log: vec![
            log("action", "decrease_allowance"),
            log("owner", deps.api.human_address(owner_raw)?),
            log("spender", spender),
            log("denomination", amount.denom),
            log("amount", amount.amount),
        ],
        data: None,
    };
    Ok(res)
}

pub fn handle_set_permissions<S: Storage, A: Api, Q: Querier, T>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    spender: HumanAddr,
    perm: Permissions,
) -> StdResult<HandleResponse<T>>
    where
        T: Clone + fmt::Debug + PartialEq + JsonSchema,
{
    let cfg = admin_list_read(&deps.storage).load()?;
    let spender_raw = &deps.api.canonical_address(&spender)?;
    let owner_raw = &deps.api.canonical_address(&env.message.sender)?;

    if !cfg.is_admin(&owner_raw) {
        return Err(StdError::unauthorized());
    }
    if spender_raw == owner_raw {
        return Err(StdError::generic_err("Cannot set allowance to own account"));
    }
    permissions(&mut deps.storage).save(spender_raw.as_slice(), &perm)?;

    let res = HandleResponse {
        messages: vec![],
        log: vec![
            log("action", "setup_permissions"),
            log("owner", deps.api.human_address(owner_raw)?),
            log("spender", spender),
            log("permissions", perm),
        ],
        data: None,
    };
    Ok(res)
}

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::AdminList {} => to_binary(&query_admin_list(deps)?),
        QueryMsg::Allowance { spender } => to_binary(&query_allowance(deps, spender)?),
        QueryMsg::CanSend { sender, msg } => to_binary(&query_can_send(deps, sender, msg)?),
        QueryMsg::AllAllowances { start_after, limit } => {
            to_binary(&query_all_allowances(deps, start_after, limit)?)
        }
    }
}

// if the subkey has no allowance, return an empty struct (not an error)
pub fn query_allowance<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    spender: HumanAddr,
) -> StdResult<Allowance> {
    let subkey = deps.api.canonical_address(&spender)?;
    let allow = allowances_read(&deps.storage)
        .may_load(subkey.as_slice())?
        .unwrap_or_default();
    Ok(allow)
}

fn query_can_send<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    sender: HumanAddr,
    msg: CosmosMsg,
) -> StdResult<CanSendResponse> {
    Ok(CanSendResponse {
        can_send: can_send(deps, sender, msg)?,
    })
}

// this can just return booleans and the query_can_send wrapper creates the struct once, not on every path
fn can_send<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    sender: HumanAddr,
    msg: CosmosMsg,
) -> StdResult<bool> {
    let owner_raw = deps.api.canonical_address(&sender)?;
    let cfg = admin_list_read(&deps.storage).load()?;
    if cfg.is_admin(&owner_raw) {
        return Ok(true);
    }

    // we only accept bank messages - ensure type and get the amount
    let amount = match msg {
        CosmosMsg::Bank(BankMsg::Send { amount, .. }) => amount,
        _ => return Ok(false),
    };

    // now we check if there is enough allowance for this message
    let allowance = allowances_read(&deps.storage).may_load(owner_raw.as_slice())?;
    match allowance {
        // if there is an allowance, we subtract the requested amount to ensure it is covered (error on underflow)
        Some(allow) => Ok(allow.balance.sub(amount).is_ok()),
        None => Ok(false),
    }
}

const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;

fn calc_limit(request: Option<u32>) -> usize {
    request.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize
}

// this will set the first key after the provided key, by appending a 1 byte
fn calc_range_start(start_after: Option<HumanAddr>) -> Option<Vec<u8>> {
    match start_after {
        Some(human) => {
            let mut v = Vec::from(human.0);
            v.push(1);
            Some(v)
        }
        None => None,
    }
}

// return a list of all allowances here
pub fn query_all_allowances<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    start_after: Option<HumanAddr>,
    limit: Option<u32>,
) -> StdResult<AllAllowancesResponse> {
    let limit = calc_limit(limit);
    let range_start = calc_range_start(start_after);

    let api = &deps.api;
    let res: StdResult<Vec<AllowanceInfo>> = allowances_read(&deps.storage)
        .range(range_start.as_deref(), None, Order::Ascending)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::balance::Balance;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, MOCK_CONTRACT_ADDR};
    use cosmwasm_std::{coin, coins, StakingMsg};
    use cw1_whitelist::msg::AdminListResponse;
    use cw2::{get_contract_version, ContractVersion};
    use crate::state::Permissions;

    // this will set up the init for other tests
    fn setup_test_case<S: Storage, A: Api, Q: Querier>(
        mut deps: &mut Extern<S, A, Q>,
        env: &Env,
        admins: &[HumanAddr],
        spenders: &[HumanAddr],
        allowances: &[Coin],
        expirations: &[Expiration],
        permissions: &[Permissions],
    ) {
        // Init a contract with admins
        let init_msg = InitMsg {
            admins: admins.to_vec(),
            mutable: true,
        };
        init(deps, env.clone(), init_msg).unwrap();

        // Add subkeys with initial allowances and permissions
        for ((spender, expiration), permissions) in spenders.iter().zip(expirations).zip(permissions) {
            for amount in allowances {
                let msg = HandleMsg::IncreaseAllowance {
                    spender: spender.clone(),
                    amount: amount.clone(),
                    expires: Some(expiration.clone()),
                };
                handle(&mut deps, env.clone(), msg).unwrap();
                let permission_msg = HandleMsg::SetPermissions {
                    spender: spender.clone(),
                    permissions: permissions.clone(),
                };
                handle(&mut deps, env.clone(), permission_msg).unwrap();
            }
        }
    }

    #[test]
    fn get_contract_version_works() {
        let mut deps = mock_dependencies(20, &[]);

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
        let initial_permissions = vec![Permissions::default(), Permissions::default()];

        let env = mock_env(owner, &[]);
        setup_test_case(
            &mut deps,
            &env,
            &admins,
            &initial_spenders,
            &initial_allowances,
            &initial_expirations,
            &initial_permissions,
        );

        assert_eq!(
            ContractVersion {
                contract: CONTRACT_NAME.to_string(),
                version: CONTRACT_VERSION.to_string(),
            },
            get_contract_version(&deps.storage).unwrap()
        )
    }

    #[test]
    fn query_allowance_works() {
        let mut deps = mock_dependencies(20, &[]);

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
        let initial_permissions = vec![Permissions::default(), Permissions::default()];

        let env = mock_env(owner, &[]);
        setup_test_case(
            &mut deps,
            &env,
            &admins,
            &initial_spenders,
            &initial_allowances,
            &initial_expirations,
            &initial_permissions,
        );

        // Check allowances work for accounts with balances
        let allowance = query_allowance(&deps, spender1.clone()).unwrap();
        assert_eq!(
            allowance,
            Allowance {
                balance: Balance(vec![allow1.clone()]),
                expires: expires_never.clone(),
            }
        );
        let allowance = query_allowance(&deps, spender2.clone()).unwrap();
        assert_eq!(
            allowance,
            Allowance {
                balance: Balance(vec![allow1.clone()]),
                expires: expires_never.clone(),
            }
        );

        // Check allowances work for accounts with no balance
        let allowance = query_allowance(&deps, spender3.clone()).unwrap();
        assert_eq!(allowance, Allowance::default(),);
    }

    #[test]
    fn query_all_allowances_works() {
        let mut deps = mock_dependencies(20, &[]);

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
        let initial_permissions = vec![Permissions::default(), Permissions::default(), Permissions::default()];

        let env = mock_env(owner, &[]);
        setup_test_case(
            &mut deps,
            &env,
            &admins,
            &initial_spenders,
            &initial_allowances,
            &initial_expirations,
            &initial_permissions,
        );


        // let's try pagination
        let allowances = query_all_allowances(&deps, None, Some(2))
            .unwrap()
            .allowances;
        assert_eq!(2, allowances.len());
        assert_eq!(
            allowances[0],
            AllowanceInfo {
                spender: spender1,
                balance: Balance(initial_allowances.clone()),
                expires: Expiration::Never {},
            }
        );
        assert_eq!(
            allowances[1],
            AllowanceInfo {
                spender: spender2.clone(),
                balance: Balance(initial_allowances.clone()),
                expires: Expiration::Never {},
            }
        );

        // now continue from after the last one
        let allowances = query_all_allowances(&deps, Some(spender2), Some(2))
            .unwrap()
            .allowances;
        assert_eq!(1, allowances.len());
        assert_eq!(
            allowances[0],
            AllowanceInfo {
                spender: spender3,
                balance: Balance(initial_allowances.clone()),
                expires: expires_later,
            }
        );
    }

    #[test]
    fn update_admins_and_query() {
        let mut deps = mock_dependencies(20, &[]);

        let owner = HumanAddr::from("admin0001");
        let admin2 = HumanAddr::from("admin0002");
        let admin3 = HumanAddr::from("admin0003");
        let initial_admins = vec![owner.clone(), admin2.clone()];

        let env = mock_env(owner.clone(), &[]);
        setup_test_case(&mut deps, &env, &initial_admins, &vec![], &vec![], &vec![], &vec![]);

        // Verify
        let config = query_admin_list(&deps).unwrap();
        assert_eq!(
            config,
            AdminListResponse {
                admins: initial_admins.clone(),
                mutable: true,
            }
        );

        // Add a third (new) admin
        let new_admins = vec![owner.clone(), admin2.clone(), admin3.clone()];
        let msg = HandleMsg::UpdateAdmins {
            admins: new_admins.clone(),
        };
        handle(&mut deps, env.clone(), msg).unwrap();

        // Verify
        let config = query_admin_list(&deps).unwrap();
        println!("config: {:#?}", config);
        assert_eq!(
            config,
            AdminListResponse {
                admins: new_admins,
                mutable: true,
            }
        );

        // Set admin3 as the only admin
        let msg = HandleMsg::UpdateAdmins {
            admins: vec![admin3.clone()],
        };
        handle(&mut deps, env.clone(), msg).unwrap();

        // Verify admin3 is now the sole admin
        let config = query_admin_list(&deps).unwrap();
        println!("config: {:#?}", config);
        assert_eq!(
            config,
            AdminListResponse {
                admins: vec![admin3.clone()],
                mutable: true,
            }
        );

        // Try to add owner back
        let msg = HandleMsg::UpdateAdmins {
            admins: vec![admin3.clone(), owner.clone()],
        };
        let res = handle(&mut deps, env.clone(), msg);

        // Verify it fails (admin3 is now the owner)
        assert!(res.is_err());

        // Connect as admin3
        let env = mock_env(admin3.clone(), &[]);
        // Add owner back
        let msg = HandleMsg::UpdateAdmins {
            admins: vec![admin3.clone(), owner.clone()],
        };
        handle(&mut deps, env.clone(), msg).unwrap();

        // Verify
        let config = query_admin_list(&deps).unwrap();
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
        let initial_permissions = vec![Permissions::default(), Permissions::default()];


        let env = mock_env(owner, &[]);
        setup_test_case(
            &mut deps,
            &env,
            &admins,
            &initial_spenders,
            &initial_allowances,
            &initial_expirations,
            &initial_permissions,
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
                balance: Balance(vec![coin(amount1 * 2, &allow1.denom), allow2.clone()]),
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
                balance: Balance(vec![allow1.clone(), allow2.clone(), allow3.clone()]),
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
                balance: Balance(vec![allow1.clone()]),
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
                balance: Balance(vec![allow2.clone()]),
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
        let initial_permissions = vec![Permissions::default(), Permissions::default()];

        let env = mock_env(owner, &[]);
        setup_test_case(
            &mut deps,
            &env,
            &admins,
            &initial_spenders,
            &initial_allowances,
            &initial_expirations,
            &initial_permissions,
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
                balance: Balance(vec![allow1.clone(), allow2.clone()]),
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
                balance: Balance(vec![allow1.clone()]),
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
                balance: Balance(vec![
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
                balance: Balance(vec![allow2]),
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
        let initial_permissions = vec![Permissions::default()];

        let env = mock_env(owner.clone(), &[]);
        setup_test_case(
            &mut deps,
            &env,
            &admins,
            &initial_spenders,
            &initial_allowances,
            &initial_expirations,
            &initial_permissions,
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
    fn test_permission_checks() {
        let mut deps = mock_dependencies(20, &[]);

        let owner = HumanAddr::from("admin0001");
        let admins = vec![owner.clone()];

        // spender1 has every permission to stake
        let spender1 = HumanAddr::from("spender0001");
        // spender2 do not have permission
        let spender2 = HumanAddr::from("spender0002");
        let initial_spenders = vec![spender1.clone(), spender2.clone()];

        let denom = "token1";
        let amount = 10000;
        let allow = coin(amount, denom);
        let initial_allowances = vec![allow.clone(), allow.clone()];

        let expires_never = Expiration::Never {};
        let initial_expirations = vec![expires_never.clone(), expires_never.clone()];
        let god_mode = Permissions {
            delegate: true,
            redelegate: true,
            undelegate: true,
            withdraw: true,
        };
        let initial_permissions = vec![god_mode.clone(), Permissions::default()];

        let env = mock_env(owner.clone(), &[]);
        setup_test_case(
            &mut deps,
            &env,
            &admins,
            &initial_spenders,
            &initial_allowances,
            &initial_expirations,
            &initial_permissions,
        );

        let msg_delegate = vec![StakingMsg::Delegate{
            validator: HumanAddr(String::from("validator")),
            amount: allow.clone(),
        }.into()];
        let msg_redelegate = vec![StakingMsg::Redelegate{
            src_validator: Default::default(),
            dst_validator: Default::default(),
            amount: allow.clone(),
        }.into()];
        let msg_undelegate= vec![StakingMsg::Undelegate{
            validator: HumanAddr(String::from("validator")),
            amount: allow.clone(),
        }.into()];
        let msg_withdraw = vec![StakingMsg::Withdraw{
            validator: Default::default(),
            recipient: None,
        }.into()];

        let msgs = vec![
            msg_delegate,
            msg_redelegate,
            msg_undelegate,
            msg_withdraw,
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
            &[Permissions::default()],
        );

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
        let staking_msg = CosmosMsg::Staking(StakingMsg::Delegate {
            validator: anyone.clone(),
            amount: coin(70000, "ureef"),
        });

        // owner can send big or small
        let res = query_can_send(&deps, owner.clone(), send_msg.clone()).unwrap();
        assert_eq!(res.can_send, true);
        let res = query_can_send(&deps, owner.clone(), send_msg_large.clone()).unwrap();
        assert_eq!(res.can_send, true);
        // owner can stake
        let res = query_can_send(&deps, owner.clone(), staking_msg.clone()).unwrap();
        assert_eq!(res.can_send, true);

        // spender can send small
        let res = query_can_send(&deps, spender.clone(), send_msg.clone()).unwrap();
        assert_eq!(res.can_send, true);
        // not too big
        let res = query_can_send(&deps, spender.clone(), send_msg_large.clone()).unwrap();
        assert_eq!(res.can_send, false);
        // and not stake
        let res = query_can_send(&deps, spender.clone(), staking_msg.clone()).unwrap();
        assert_eq!(res.can_send, false);

        // random person cannot do anything
        let res = query_can_send(&deps, anyone.clone(), send_msg.clone()).unwrap();
        assert_eq!(res.can_send, false);
        let res = query_can_send(&deps, anyone.clone(), send_msg_large.clone()).unwrap();
        assert_eq!(res.can_send, false);
        let res = query_can_send(&deps, anyone.clone(), staking_msg.clone()).unwrap();
        assert_eq!(res.can_send, false);
    }
}
