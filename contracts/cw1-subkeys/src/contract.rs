use schemars::JsonSchema;
use std::fmt;
use std::ops::{AddAssign, Sub};

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, to_binary, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, DistributionMsg, Empty, Env,
    MessageInfo, Order, Response, StakingMsg, StdError, StdResult, SubMsg,
};
use cw0::Expiration;
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
use cw_storage_plus::Bound;

use crate::error::ContractError;
use crate::msg::{
    AllAllowancesResponse, AllPermissionsResponse, AllowanceInfo, ExecuteMsg, PermissionsInfo,
    QueryMsg,
};
use crate::state::{Allowance, Permissions, ALLOWANCES, PERMISSIONS};

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
    // Wrap `msgs` in SubMsg.
    let msgs: Vec<_> = msgs.into_iter().map(SubMsg::new).collect();
    let cfg = ADMIN_LIST.load(deps.storage)?;

    // Not an admin - need to check for permissions
    if !cfg.is_admin(info.sender.as_ref()) {
        for msg in &msgs {
            match &msg.msg {
                CosmosMsg::Staking(staking_msg) => {
                    let perm = PERMISSIONS.may_load(deps.storage, &info.sender)?;
                    let perm = perm.ok_or(ContractError::NotAllowed {})?;
                    check_staking_permissions(staking_msg, perm)?;
                }
                CosmosMsg::Distribution(distribution_msg) => {
                    let perm = PERMISSIONS.may_load(deps.storage, &info.sender)?;
                    let perm = perm.ok_or(ContractError::NotAllowed {})?;
                    check_distribution_permissions(distribution_msg, perm)?;
                }
                CosmosMsg::Bank(BankMsg::Send {
                    to_address: _,
                    amount,
                }) => {
                    ALLOWANCES.update::<_, ContractError>(deps.storage, &info.sender, |allow| {
                        let mut allowance = allow.ok_or(ContractError::NoAllowance {})?;
                        // Decrease allowance
                        allowance.balance = allowance.balance.sub(amount.clone())?;
                        Ok(allowance)
                    })?;
                }
                _ => {
                    return Err(ContractError::MessageTypeRejected {});
                }
            }
        }
    }
    // Relay messages
    let res = Response {
        messages: msgs,
        attributes: vec![attr("action", "execute"), attr("owner", info.sender)],
        ..Response::default()
    };
    Ok(res)
}

pub fn check_staking_permissions(
    staking_msg: &StakingMsg,
    permissions: Permissions,
) -> Result<(), ContractError> {
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
        s => panic!("Unsupported staking message: {:?}", s),
    }
    Ok(())
}

pub fn check_distribution_permissions(
    distribution_msg: &DistributionMsg,
    permissions: Permissions,
) -> Result<(), ContractError> {
    match distribution_msg {
        DistributionMsg::SetWithdrawAddress { .. } => {
            if !permissions.withdraw {
                return Err(ContractError::WithdrawAddrPerm {});
            }
        }
        DistributionMsg::WithdrawDelegatorReward { .. } => {
            if !permissions.withdraw {
                return Err(ContractError::WithdrawPerm {});
            }
        }
        s => panic!("Unsupported distribution message: {:?}", s),
    }
    Ok(())
}

pub fn execute_increase_allowance<T>(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    spender: String,
    amount: Coin,
    expires: Option<Expiration>,
) -> Result<Response<T>, ContractError>
where
    T: Clone + fmt::Debug + PartialEq + JsonSchema,
{
    let cfg = ADMIN_LIST.load(deps.storage)?;
    if !cfg.is_admin(info.sender.as_ref()) {
        return Err(ContractError::Unauthorized {});
    }

    let spender_addr = deps.api.addr_validate(&spender)?;
    if info.sender == spender_addr {
        return Err(ContractError::CannotSetOwnAccount {});
    }

    ALLOWANCES.update::<_, StdError>(deps.storage, &spender_addr, |allow| {
        let mut allowance = allow.unwrap_or_default();
        if let Some(exp) = expires {
            allowance.expires = exp;
        }
        allowance.balance.add_assign(amount.clone());
        Ok(allowance)
    })?;

    let res = Response {
        attributes: vec![
            attr("action", "increase_allowance"),
            attr("owner", info.sender),
            attr("spender", spender),
            attr("denomination", amount.denom),
            attr("amount", amount.amount),
        ],
        ..Response::default()
    };
    Ok(res)
}

pub fn execute_decrease_allowance<T>(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    spender: String,
    amount: Coin,
    expires: Option<Expiration>,
) -> Result<Response<T>, ContractError>
where
    T: Clone + fmt::Debug + PartialEq + JsonSchema,
{
    let cfg = ADMIN_LIST.load(deps.storage)?;
    if !cfg.is_admin(info.sender.as_ref()) {
        return Err(ContractError::Unauthorized {});
    }

    let spender_addr = deps.api.addr_validate(&spender)?;
    if info.sender == spender_addr {
        return Err(ContractError::CannotSetOwnAccount {});
    }

    let allowance =
        ALLOWANCES.update::<_, ContractError>(deps.storage, &spender_addr, |allow| {
            // Fail fast
            let mut allowance = allow.ok_or(ContractError::NoAllowance {})?;
            if let Some(exp) = expires {
                allowance.expires = exp;
            }
            allowance.balance = allowance.balance.sub_saturating(amount.clone())?; // Tolerates underflows (amount bigger than balance), but fails if there are no tokens at all for the denom (report potential errors)
            Ok(allowance)
        })?;
    if allowance.balance.is_empty() {
        ALLOWANCES.remove(deps.storage, &spender_addr);
    }

    let res = Response {
        attributes: vec![
            attr("action", "decrease_allowance"),
            attr("owner", info.sender),
            attr("spender", spender),
            attr("denomination", amount.denom),
            attr("amount", amount.amount),
        ],
        ..Response::default()
    };
    Ok(res)
}

pub fn execute_set_permissions<T>(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    spender: String,
    perm: Permissions,
) -> Result<Response<T>, ContractError>
where
    T: Clone + fmt::Debug + PartialEq + JsonSchema,
{
    let cfg = ADMIN_LIST.load(deps.storage)?;
    if !cfg.is_admin(info.sender.as_ref()) {
        return Err(ContractError::Unauthorized {});
    }

    let spender_addr = deps.api.addr_validate(&spender)?;
    if info.sender == spender_addr {
        return Err(ContractError::CannotSetOwnAccount {});
    }
    PERMISSIONS.save(deps.storage, &spender_addr, &perm)?;

    let res = Response {
        attributes: vec![
            attr("action", "set_permissions"),
            attr("owner", info.sender),
            attr("spender", spender),
            attr("permissions", perm),
        ],
        ..Response::default()
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
pub fn query_allowance(deps: Deps, spender: String) -> StdResult<Allowance> {
    // we can use unchecked here as it is a query - bad value means a miss, we never write it
    let spender = deps.api.addr_validate(&spender)?;
    let allow = ALLOWANCES
        .may_load(deps.storage, &spender)?
        .unwrap_or_default();
    Ok(allow)
}

// if the subkey has no permissions, return an empty struct (not an error)
pub fn query_permissions(deps: Deps, spender: String) -> StdResult<Permissions> {
    let spender = deps.api.addr_validate(&spender)?;
    let permissions = PERMISSIONS
        .may_load(deps.storage, &spender)?
        .unwrap_or_default();
    Ok(permissions)
}

fn query_can_execute(deps: Deps, sender: String, msg: CosmosMsg) -> StdResult<CanExecuteResponse> {
    Ok(CanExecuteResponse {
        can_execute: can_execute(deps, sender, msg)?,
    })
}

// this can just return booleans and the query_can_execute wrapper creates the struct once, not on every path
fn can_execute(deps: Deps, sender: String, msg: CosmosMsg) -> StdResult<bool> {
    let cfg = ADMIN_LIST.load(deps.storage)?;
    if cfg.is_admin(&sender) {
        return Ok(true);
    }

    let sender = deps.api.addr_validate(&sender)?;
    match msg {
        CosmosMsg::Bank(BankMsg::Send { amount, .. }) => {
            // now we check if there is enough allowance for this message
            let allowance = ALLOWANCES.may_load(deps.storage, &sender)?;
            match allowance {
                // if there is an allowance, we subtract the requested amount to ensure it is covered (error on underflow)
                Some(allow) => Ok(allow.balance.sub(amount).is_ok()),
                None => Ok(false),
            }
        }
        CosmosMsg::Staking(staking_msg) => {
            let perm_opt = PERMISSIONS.may_load(deps.storage, &sender)?;
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
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<AllAllowancesResponse> {
    let limit = calc_limit(limit);
    // we use raw addresses here....
    let start = start_after.map(Bound::exclusive);

    let res: StdResult<Vec<AllowanceInfo>> = ALLOWANCES
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            item.and_then(|(k, allow)| {
                Ok(AllowanceInfo {
                    spender: String::from_utf8(k)?,
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
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<AllPermissionsResponse> {
    let limit = calc_limit(limit);
    let start = start_after.map(Bound::exclusive);

    let res: StdResult<Vec<PermissionsInfo>> = PERMISSIONS
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            item.and_then(|(k, perm)| {
                Ok(PermissionsInfo {
                    spender: String::from_utf8(k)?,
                    permissions: perm,
                })
            })
        })
        .collect();
    Ok(AllPermissionsResponse { permissions: res? })
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::{
        mock_dependencies, mock_env, mock_info, MockApi, MockQuerier, MockStorage,
    };
    use cosmwasm_std::{coin, coins, Addr, OwnedDeps, StakingMsg, Timestamp};

    use cw0::NativeBalance;
    use cw1_whitelist::msg::AdminListResponse;
    use cw2::{get_contract_version, ContractVersion};

    use crate::state::Permissions;

    use super::*;

    const OWNER: &str = "owner";

    const ADMIN1: &str = "admin1";
    const ADMIN2: &str = "admin2";

    const SPENDER1: &str = "spender1";
    const SPENDER2: &str = "spender2";
    const SPENDER3: &str = "spender3";
    const SPENDER4: &str = "spender4";

    const TOKEN: &str = "token";
    const TOKEN1: &str = "token1";
    const TOKEN2: &str = "token2";
    const TOKEN3: &str = "token3";

    const ALL_PERMS: Permissions = Permissions {
        delegate: true,
        redelegate: true,
        undelegate: true,
        withdraw: true,
    };
    const NO_PERMS: Permissions = Permissions {
        delegate: false,
        redelegate: false,
        undelegate: false,
        withdraw: false,
    };

    /// Helper structure for Suite configuration
    #[derive(Default)]
    struct SuiteConfig {
        spenders: Vec<Spender>,
        admins: Vec<&'static str>,
    }

    impl SuiteConfig {
        fn init(self) -> Suite {
            Suite::init_with_config(self)
        }
    }

    #[derive(Default)]
    struct Spender {
        spender: &'static str,
        allowances: Vec<Coin>,
        allowances_expire: Option<Expiration>,
        permissions: Option<Permissions>,
    }

    /// Test suite helper unifying test initialization, keeping access to created data
    struct Suite {
        deps: OwnedDeps<MockStorage, MockApi, MockQuerier>,
        owner_info: MessageInfo,
    }

    impl Suite {
        /// Initializes test case using default config
        fn init() -> Self {
            Self::init_with_config(SuiteConfig::default())
        }

        /// Initialized test case using provided config
        fn init_with_config(config: SuiteConfig) -> Self {
            let mut deps = mock_dependencies(&[]);
            let admins = std::iter::once(OWNER)
                .chain(config.admins)
                .map(ToOwned::to_owned)
                .collect();

            let instantiate_msg = InstantiateMsg {
                admins,
                mutable: true,
            };
            let owner_info = mock_info(OWNER, &[]);

            instantiate(
                deps.as_mut().branch(),
                mock_env(),
                owner_info.clone(),
                instantiate_msg,
            )
            .unwrap();

            for Spender {
                spender,
                allowances,
                allowances_expire: expires,
                permissions,
            } in config.spenders
            {
                for amount in allowances {
                    let msg = ExecuteMsg::IncreaseAllowance {
                        spender: spender.to_owned(),
                        amount,
                        expires,
                    };
                    execute(deps.as_mut().branch(), mock_env(), owner_info.clone(), msg).unwrap();
                }

                if let Some(permissions) = permissions {
                    let msg = ExecuteMsg::SetPermissions {
                        spender: spender.to_owned(),
                        permissions,
                    };
                    execute(deps.as_mut().branch(), mock_env(), owner_info.clone(), msg).unwrap();
                }
            }

            Self { deps, owner_info }
        }
    }

    /// Helper function for comparing vectors or another slice-like object as they would represent
    /// set with duplications. Compares sets by first sorting elements usning provided ordering.
    /// This functions reshufless elements inplace, as it should never matter as compared
    /// containers should represent same value regardless of ordering, and making this inplace just
    /// safes obsolete copying.
    ///
    /// This is implemented as a macro instead of function to throw panic in the place of macro
    /// usage instead of from function called inside test.
    macro_rules! assert_sorted_eq {
        ($left:expr, $right:expr, $cmp:expr $(,)?) => {
            let mut left = $left;
            left.sort_by(&$cmp);

            let mut right = $right;
            right.sort_by($cmp);

            assert_eq!(left, right);
        };
    }

    // this will set up instantiation for other tests
    fn setup_test_case(
        mut deps: DepsMut,
        info: &MessageInfo,
        admins: &[&str],
        spenders: &[&str],
        allowances: &[Coin],
        expirations: &[Expiration],
    ) {
        // Instantiate a contract with admins
        let instantiate_msg = InstantiateMsg {
            admins: admins.iter().map(|x| x.to_string()).collect(),
            mutable: true,
        };
        instantiate(deps.branch(), mock_env(), info.clone(), instantiate_msg).unwrap();

        // Add subkeys with initial allowances
        for (spender, expiration) in spenders.iter().zip(expirations) {
            for amount in allowances {
                let msg = ExecuteMsg::IncreaseAllowance {
                    spender: spender.to_string(),
                    amount: amount.clone(),
                    expires: Some(*expiration),
                };
                execute(deps.branch(), mock_env(), info.clone(), msg).unwrap();
            }
        }
    }

    #[test]
    fn get_contract_version_works() {
        let Suite { deps, .. } = Suite::init();

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
        let Suite { deps, .. } = SuiteConfig {
            spenders: vec![
                Spender {
                    spender: SPENDER1,
                    allowances: vec![coin(1111, TOKEN)],
                    ..Spender::default()
                },
                Spender {
                    spender: SPENDER2,
                    allowances: vec![coin(2222, TOKEN)],
                    ..Spender::default()
                },
            ],
            ..SuiteConfig::default()
        }
        .init();

        // Check allowances work for accounts with balances
        let allowance = query_allowance(deps.as_ref(), SPENDER1.to_owned()).unwrap();
        assert_eq!(
            allowance,
            Allowance {
                balance: NativeBalance(vec![coin(1111, TOKEN)]),
                expires: Expiration::Never {},
            }
        );
        let allowance = query_allowance(deps.as_ref(), SPENDER2.to_owned()).unwrap();
        assert_eq!(
            allowance,
            Allowance {
                balance: NativeBalance(vec![coin(2222, TOKEN)]),
                expires: Expiration::Never {},
            }
        );

        // Check allowances work for accounts with no balance
        let allowance = query_allowance(deps.as_ref(), SPENDER3.to_string()).unwrap();
        assert_eq!(allowance, Allowance::default());
    }

    #[test]
    fn query_all_allowances_works() {
        let Suite { deps, .. } = SuiteConfig {
            spenders: vec![
                Spender {
                    spender: SPENDER1,
                    allowances: vec![coin(1234, TOKEN)],
                    ..Spender::default()
                },
                Spender {
                    spender: SPENDER2,
                    allowances: vec![coin(2345, TOKEN)],
                    allowances_expire: Some(Expiration::Never {}),
                    ..Spender::default()
                },
                Spender {
                    spender: SPENDER3,
                    allowances: vec![coin(3456, TOKEN)],
                    allowances_expire: Some(Expiration::AtHeight(12345)),
                    ..Spender::default()
                },
            ],
            ..SuiteConfig::default()
        }
        .init();

        // let's try pagination.
        //
        // Check is tricky, as there is no guarantee about order expiration are received (as it is
        // dependent at least on ordering of insertions), so to check if pagination works, all what
        // can we do is to ensure parts are of expected size, and that collectively all allowances
        // are returned.
        let batch1 = query_all_allowances(deps.as_ref(), None, Some(2))
            .unwrap()
            .allowances;
        assert_eq!(2, batch1.len());

        // now continue from after the last one
        let batch2 = query_all_allowances(deps.as_ref(), Some(batch1[1].spender.clone()), Some(2))
            .unwrap()
            .allowances;
        assert_eq!(1, batch2.len());

        let expected = vec![
            AllowanceInfo {
                spender: SPENDER1.to_owned(),
                balance: NativeBalance(vec![coin(1234, TOKEN)]),
                expires: Expiration::Never {},
            },
            AllowanceInfo {
                spender: SPENDER2.to_owned(),
                balance: NativeBalance(vec![coin(2345, TOKEN)]),
                expires: Expiration::Never {},
            },
            AllowanceInfo {
                spender: SPENDER3.to_owned(),
                balance: NativeBalance(vec![coin(3456, TOKEN)]),
                expires: Expiration::AtHeight(12345),
            },
        ];

        assert_sorted_eq!(
            expected,
            [batch1, batch2].concat(),
            |l: &AllowanceInfo, r: &AllowanceInfo| l.spender.cmp(&r.spender)
        );
    }

    #[test]
    fn query_permissions_works() {
        let Suite { deps, .. } = SuiteConfig {
            spenders: vec![
                Spender {
                    spender: SPENDER1,
                    permissions: Some(ALL_PERMS),
                    ..Spender::default()
                },
                Spender {
                    spender: SPENDER2,
                    permissions: Some(NO_PERMS),
                    ..Spender::default()
                },
            ],
            ..SuiteConfig::default()
        }
        .init();

        let permissions = query_permissions(deps.as_ref(), SPENDER1.to_string()).unwrap();
        assert_eq!(permissions, ALL_PERMS);

        let permissions = query_permissions(deps.as_ref(), SPENDER2.to_string()).unwrap();
        assert_eq!(permissions, NO_PERMS);

        // no permission is set. should return false
        let permissions = query_permissions(deps.as_ref(), SPENDER3.to_string()).unwrap();
        assert_eq!(permissions, NO_PERMS);
    }

    #[test]
    fn query_all_permissions_works() {
        let Suite { deps, .. } = SuiteConfig {
            spenders: vec![
                Spender {
                    spender: SPENDER1,
                    permissions: Some(ALL_PERMS),
                    ..Spender::default()
                },
                Spender {
                    spender: SPENDER2,
                    permissions: Some(NO_PERMS),
                    ..Spender::default()
                },
                Spender {
                    spender: SPENDER3,
                    permissions: Some(NO_PERMS),
                    ..Spender::default()
                },
            ],
            ..SuiteConfig::default()
        }
        .init();

        // let's try pagination
        let batch1 = query_all_permissions(deps.as_ref(), None, Some(2))
            .unwrap()
            .permissions;
        assert_eq!(batch1.len(), 2);

        let batch2 = query_all_permissions(deps.as_ref(), Some(batch1[1].spender.clone()), Some(2))
            .unwrap()
            .permissions;
        assert_eq!(batch2.len(), 1);

        let expected = vec![
            PermissionsInfo {
                spender: SPENDER1.to_owned(),
                permissions: ALL_PERMS,
            },
            PermissionsInfo {
                spender: SPENDER2.to_owned(),
                permissions: NO_PERMS,
            },
            PermissionsInfo {
                spender: SPENDER3.to_owned(),
                permissions: NO_PERMS,
            },
        ];

        assert_sorted_eq!(
            [batch1, batch2].concat(),
            expected,
            |l: &PermissionsInfo, r: &PermissionsInfo| l.spender.cmp(&r.spender)
        );
    }

    #[test]
    fn update_admins_and_query() {
        let Suite {
            mut deps,
            owner_info,
            ..
        } = SuiteConfig {
            admins: vec![ADMIN1],
            ..SuiteConfig::default()
        }
        .init();

        // Verify
        assert_eq!(
            query_admin_list(deps.as_ref()).unwrap().canonical(),
            AdminListResponse {
                admins: vec![OWNER.to_owned(), ADMIN1.to_owned()],
                mutable: true,
            }
            .canonical()
        );

        // Add a third (new) admin
        let new_admins = vec![OWNER.to_owned(), ADMIN1.to_owned(), ADMIN2.to_owned()];
        let msg = ExecuteMsg::UpdateAdmins {
            admins: new_admins.clone(),
        };
        execute(deps.as_mut(), mock_env(), owner_info.clone(), msg).unwrap();

        // Verify
        assert_eq!(
            query_admin_list(deps.as_ref()).unwrap().canonical(),
            AdminListResponse {
                admins: new_admins,
                mutable: true,
            }
            .canonical()
        );

        // Set ADMIN2 as the only admin
        let msg = ExecuteMsg::UpdateAdmins {
            admins: vec![ADMIN2.to_string()],
        };
        execute(deps.as_mut(), mock_env(), owner_info.clone(), msg).unwrap();

        // Verify admin3 is now the sole admin
        assert_eq!(
            query_admin_list(deps.as_ref()).unwrap().canonical(),
            AdminListResponse {
                admins: vec![ADMIN2.to_owned()],
                mutable: true,
            }
            .canonical()
        );

        // Try to add owner back
        let msg = ExecuteMsg::UpdateAdmins {
            admins: vec![ADMIN2.to_owned(), OWNER.to_owned()],
        };
        let res = execute(deps.as_mut(), mock_env(), owner_info, msg);

        // Verify it fails (ADMIN2 is now the owner)
        assert!(res.is_err());

        // Connect as admin2
        let info = mock_info(ADMIN2, &[]);
        // Add owner back
        let msg = ExecuteMsg::UpdateAdmins {
            admins: vec![ADMIN2.to_owned(), OWNER.to_owned()],
        };
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Verify
        assert_eq!(
            query_admin_list(deps.as_ref()).unwrap().canonical(),
            AdminListResponse {
                admins: vec![ADMIN2.to_owned(), OWNER.to_owned()],
                mutable: true,
            }
            .canonical()
        );
    }

    #[test]
    fn increase_allowances() {
        let Suite {
            mut deps,
            owner_info,
            ..
        } = SuiteConfig {
            spenders: vec![
                Spender {
                    spender: SPENDER1,
                    allowances: vec![coin(1111, TOKEN1), coin(2222, TOKEN2)],
                    allowances_expire: Some(Expiration::AtHeight(5432)),
                    ..Spender::default()
                },
                Spender {
                    spender: SPENDER2,
                    allowances: vec![coin(3333, TOKEN1), coin(4444, TOKEN2)],
                    ..Spender::default()
                },
            ],
            ..SuiteConfig::default()
        }
        .init();

        // Add to spender1 account (expires = None) => don't change Expiration
        let msg = ExecuteMsg::IncreaseAllowance {
            spender: SPENDER1.to_owned(),
            amount: coin(1111, TOKEN1),
            expires: None,
        };
        execute(deps.as_mut(), mock_env(), owner_info.clone(), msg).unwrap();

        // Verify
        assert_eq!(
            query_allowance(deps.as_ref(), SPENDER1.to_string())
                .unwrap()
                .canonical(),
            Allowance {
                balance: NativeBalance(vec![coin(2222, TOKEN1), coin(2222, TOKEN2)]),
                expires: Expiration::AtHeight(5432),
            }
            .canonical()
        );

        // Add to spender2 account (expires = Some)
        let msg = ExecuteMsg::IncreaseAllowance {
            spender: SPENDER2.to_owned(),
            amount: coin(5555, TOKEN3),
            expires: Some(Expiration::AtHeight(1234)),
        };
        execute(deps.as_mut(), mock_env(), owner_info.clone(), msg).unwrap();

        // Verify
        assert_eq!(
            query_allowance(deps.as_ref(), SPENDER2.to_owned())
                .unwrap()
                .canonical(),
            Allowance {
                balance: NativeBalance(vec![
                    coin(3333, TOKEN1),
                    coin(4444, TOKEN2),
                    coin(5555, TOKEN3)
                ]),
                expires: Expiration::AtHeight(1234),
            }
            .canonical()
        );

        // Add to spender3 (new account) (expires = None) => default Expiration::Never
        let msg = ExecuteMsg::IncreaseAllowance {
            spender: SPENDER3.to_string(),
            amount: coin(1111, TOKEN1),
            expires: None,
        };
        execute(deps.as_mut(), mock_env(), owner_info.clone(), msg).unwrap();

        // Verify
        assert_eq!(
            query_allowance(deps.as_ref(), SPENDER3.to_string())
                .unwrap()
                .canonical(),
            Allowance {
                balance: NativeBalance(vec![coin(1111, TOKEN1)]),
                expires: Expiration::Never {},
            }
            .canonical()
        );

        // Add to spender4 (new account) (expires = Some)
        let msg = ExecuteMsg::IncreaseAllowance {
            spender: SPENDER4.into(),
            amount: coin(2222, TOKEN2),
            expires: Some(Expiration::AtTime(Timestamp::from_seconds(1234567890))),
        };
        execute(deps.as_mut(), mock_env(), owner_info, msg).unwrap();

        // Verify
        assert_eq!(
            query_allowance(deps.as_ref(), SPENDER4.to_owned())
                .unwrap()
                .canonical(),
            Allowance {
                balance: NativeBalance(vec![coin(2222, TOKEN2)]),
                expires: Expiration::AtTime(Timestamp::from_seconds(1234567890)),
            }
        );
    }

    #[test]
    fn decrease_allowances() {
        let mut deps = mock_dependencies(&[]);

        let owner = "admin0001";
        let admins = vec![owner, "admin0002"];

        let spender1 = "spender0001";
        let spender2 = "spender0002";
        let initial_spenders = vec![spender1, spender2];

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
        let initial_expirations = vec![expires_height, expires_never];

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
            spender: spender1.to_string(),
            amount: allow3,
            expires: None,
        };
        let res = execute(deps.as_mut(), mock_env(), info.clone(), msg);

        // Verify
        assert!(res.is_err());
        // Verify everything stays the same for that spender
        let allowance = query_allowance(deps.as_ref(), spender1.to_string()).unwrap();
        assert_eq!(
            allowance,
            Allowance {
                balance: NativeBalance(vec![allow1.clone(), allow2.clone()]),
                expires: expires_height,
            }
        );

        // Subtract from spender2 (existing) account (brings denom to 0, other denoms left)
        let msg = ExecuteMsg::DecreaseAllowance {
            spender: spender2.to_string(),
            amount: allow2.clone(),
            expires: None,
        };
        execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        // Verify
        let allowance = query_allowance(deps.as_ref(), spender2.to_string()).unwrap();
        assert_eq!(
            allowance,
            Allowance {
                balance: NativeBalance(vec![allow1.clone()]),
                expires: expires_never,
            }
        );

        // Subtract from spender1 (existing) account (brings denom to > 0)
        let msg = ExecuteMsg::DecreaseAllowance {
            spender: spender1.to_string(),
            amount: coin(amount1 / 2, denom1),
            expires: None,
        };
        execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        // Verify
        let allowance = query_allowance(deps.as_ref(), spender1.to_string()).unwrap();
        assert_eq!(
            allowance,
            Allowance {
                balance: NativeBalance(vec![
                    coin(amount1 / 2 + (amount1 & 1), denom1),
                    allow2.clone()
                ]),
                expires: expires_height,
            }
        );

        // Subtract from spender2 (existing) account (brings denom to 0, no other denoms left => should delete Allowance)
        let msg = ExecuteMsg::DecreaseAllowance {
            spender: spender2.to_string(),
            amount: allow1.clone(),
            expires: None,
        };
        execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        // Verify
        let allowance = query_allowance(deps.as_ref(), spender2.to_string()).unwrap();
        assert_eq!(allowance, Allowance::default());

        // Subtract from spender2 (empty) account (should error)
        let msg = ExecuteMsg::DecreaseAllowance {
            spender: spender2.to_string(),
            amount: allow1,
            expires: None,
        };
        let res = execute(deps.as_mut(), mock_env(), info.clone(), msg);

        // Verify
        assert!(res.is_err());

        // Subtract from spender1 (existing) account (underflows denom => should delete denom)
        let msg = ExecuteMsg::DecreaseAllowance {
            spender: spender1.to_string(),
            amount: coin(amount1 * 10, denom1),
            expires: None,
        };
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Verify
        let allowance = query_allowance(deps.as_ref(), spender1.to_string()).unwrap();
        assert_eq!(
            allowance,
            Allowance {
                balance: NativeBalance(vec![allow2]),
                expires: expires_height,
            }
        );
    }

    #[test]
    fn execute_checks() {
        let mut deps = mock_dependencies(&[]);

        let owner = "admin0001";
        let admins = vec![owner, "admin0002"];

        let spender1 = "spender0001";
        let spender2 = "spender0002";
        let initial_spenders = vec![spender1];

        let denom1 = "token1";
        let amount1 = 1111;
        let allow1 = coin(amount1, denom1);
        let initial_allowances = vec![allow1];

        let expires_never = Expiration::Never {};
        let initial_expirations = vec![expires_never];

        let info = mock_info(owner, &[]);
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
            to_address: spender2.to_string(),
            amount: coins(1000, "token1"),
        }
        .into()];

        let execute_msg = ExecuteMsg::Execute { msgs: msgs.clone() };

        // spender2 cannot spend funds (no initial allowance)
        let info = mock_info(&spender2, &[]);
        let err = execute(deps.as_mut(), mock_env(), info, execute_msg.clone()).unwrap_err();
        assert_eq!(err, ContractError::NoAllowance {});

        // But spender1 can (he has enough funds)
        let info = mock_info(&spender1, &[]);
        let res = execute(deps.as_mut(), mock_env(), info.clone(), execute_msg.clone()).unwrap();
        let msgs: Vec<_> = msgs.into_iter().map(SubMsg::new).collect();
        assert_eq!(res.messages, msgs);
        assert_eq!(
            res.attributes,
            vec![
                attr("action", "execute"),
                attr("owner", spender1.to_string())
            ]
        );

        // And then cannot (not enough funds anymore)
        let err = execute(deps.as_mut(), mock_env(), info, execute_msg.clone()).unwrap_err();
        assert!(matches!(err, ContractError::Std(StdError::Overflow { .. })));

        // Owner / admins can do anything (at the contract level)
        let info = mock_info(owner, &[]);
        let res = execute(deps.as_mut(), mock_env(), info, execute_msg).unwrap();
        assert_eq!(res.messages, msgs);
        assert_eq!(
            res.attributes,
            vec![attr("action", "execute"), attr("owner", owner)]
        );

        // For admins, even other message types are allowed
        let other_msgs = vec![CosmosMsg::Custom(Empty {})];
        let execute_msg = ExecuteMsg::Execute {
            msgs: other_msgs.clone(),
        };

        let info = mock_info(&owner, &[]);
        let res = execute(deps.as_mut(), mock_env(), info, execute_msg.clone()).unwrap();
        assert_eq!(
            res.messages,
            other_msgs.into_iter().map(SubMsg::new).collect::<Vec<_>>()
        );
        assert_eq!(
            res.attributes,
            vec![attr("action", "execute"), attr("owner", owner)]
        );

        // But not for mere mortals
        let info = mock_info(&spender1, &[]);
        let err = execute(deps.as_mut(), mock_env(), info, execute_msg).unwrap_err();
        assert_eq!(err, ContractError::MessageTypeRejected {});
    }

    #[test]
    fn staking_permission_checks() {
        let mut deps = mock_dependencies(&[]);

        let owner = "admin0001";
        let admins = vec![owner.to_string()];

        // spender1 has every permission to stake
        let spender1 = "spender0001";
        // spender2 do not have permission
        let spender2 = "spender0002";
        let denom = "token1";
        let amount = 10000;
        let coin1 = coin(amount, denom);

        let god_mode = Permissions {
            delegate: true,
            redelegate: true,
            undelegate: true,
            withdraw: true,
        };

        let info = mock_info(owner, &[]);
        // Instantiate a contract with admins
        let instantiate_msg = InstantiateMsg {
            admins,
            mutable: true,
        };
        instantiate(deps.as_mut(), mock_env(), info.clone(), instantiate_msg).unwrap();

        let setup_perm_msg1 = ExecuteMsg::SetPermissions {
            spender: spender1.to_string(),
            permissions: god_mode,
        };
        execute(deps.as_mut(), mock_env(), info.clone(), setup_perm_msg1).unwrap();

        let setup_perm_msg2 = ExecuteMsg::SetPermissions {
            spender: spender2.to_string(),
            // default is no permission
            permissions: Default::default(),
        };
        // default is no permission
        execute(deps.as_mut(), mock_env(), info.clone(), setup_perm_msg2).unwrap();

        let msg_delegate = vec![StakingMsg::Delegate {
            validator: "validator1".into(),
            amount: coin1.clone(),
        }
        .into()];
        let msg_redelegate = vec![StakingMsg::Redelegate {
            src_validator: "validator1".into(),
            dst_validator: "validator2".into(),
            amount: coin1.clone(),
        }
        .into()];
        let msg_undelegate = vec![StakingMsg::Undelegate {
            validator: "validator1".into(),
            amount: coin1,
        }
        .into()];
        let msg_withdraw = vec![DistributionMsg::WithdrawDelegatorReward {
            validator: "validator1".into(),
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
        let spender3 = "spender0003";
        let setup_perm_msg3 = ExecuteMsg::SetPermissions {
            spender: spender3.to_string(),
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
            ExecuteMsg::Execute { msgs: msg_delegate },
        );
        // FIXME need better error check here
        assert!(res.is_err());
        let res = execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            ExecuteMsg::Execute {
                msgs: msg_redelegate,
            },
        );
        assert!(res.is_ok());
        let res = execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            ExecuteMsg::Execute {
                msgs: msg_undelegate,
            },
        );
        assert!(res.is_ok());
        let res = execute(
            deps.as_mut(),
            mock_env(),
            info,
            ExecuteMsg::Execute { msgs: msg_withdraw },
        );
        assert!(res.is_err())
    }

    // tests permissions and allowances are independent features and does not affect each other
    #[test]
    fn permissions_allowances_independent() {
        let mut deps = mock_dependencies(&[]);

        let owner = "admin0001";
        let admins = vec![owner.to_string()];

        // spender1 has every permission to stake
        let spender1 = "spender0001";
        let spender2 = "spender0002";
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

        let info = mock_info(owner, &[]);
        // Instantiate a contract with admins
        let instantiate_msg = InstantiateMsg {
            admins,
            mutable: true,
        };
        instantiate(deps.as_mut(), mock_env(), info.clone(), instantiate_msg).unwrap();

        // setup permission and then allowance and check if changed
        let setup_perm_msg = ExecuteMsg::SetPermissions {
            spender: spender1.to_string(),
            permissions: perm,
        };
        execute(deps.as_mut(), mock_env(), info.clone(), setup_perm_msg).unwrap();

        let setup_allowance_msg = ExecuteMsg::IncreaseAllowance {
            spender: spender1.to_string(),
            amount: coin.clone(),
            expires: None,
        };
        execute(deps.as_mut(), mock_env(), info.clone(), setup_allowance_msg).unwrap();

        let res_perm = query_permissions(deps.as_ref(), spender1.to_string()).unwrap();
        assert_eq!(perm, res_perm);
        let res_allow = query_allowance(deps.as_ref(), spender1.to_string()).unwrap();
        assert_eq!(allow, res_allow);

        // setup allowance and then permission and check if changed
        let setup_allowance_msg = ExecuteMsg::IncreaseAllowance {
            spender: spender2.to_string(),
            amount: coin,
            expires: None,
        };
        execute(deps.as_mut(), mock_env(), info.clone(), setup_allowance_msg).unwrap();

        let setup_perm_msg = ExecuteMsg::SetPermissions {
            spender: spender2.to_string(),
            permissions: perm,
        };
        execute(deps.as_mut(), mock_env(), info, setup_perm_msg).unwrap();

        let res_perm = query_permissions(deps.as_ref(), spender2.to_string()).unwrap();
        assert_eq!(perm, res_perm);
        let res_allow = query_allowance(deps.as_ref(), spender2.to_string()).unwrap();
        assert_eq!(allow, res_allow);
    }

    #[test]
    fn can_execute_query_works() {
        let mut deps = mock_dependencies(&[]);

        let owner = "admin007";
        let spender = "spender808";
        let anyone = "anyone";

        let info = mock_info(owner, &[]);
        // spender has allowance of 55000 ushell
        setup_test_case(
            deps.as_mut(),
            &info,
            &[owner],
            &[spender],
            &coins(55000, "ushell"),
            &[Expiration::Never {}],
        );

        let perm = Permissions {
            delegate: true,
            redelegate: true,
            undelegate: false,
            withdraw: false,
        };

        let spender_addr = Addr::unchecked(spender);
        let _ = PERMISSIONS.save(&mut deps.storage, &spender_addr, &perm);

        // let us make some queries... different msg types by owner and by other
        let send_msg = CosmosMsg::Bank(BankMsg::Send {
            to_address: anyone.to_string(),
            amount: coins(12345, "ushell"),
        });
        let send_msg_large = CosmosMsg::Bank(BankMsg::Send {
            to_address: anyone.to_string(),
            amount: coins(1234567, "ushell"),
        });
        let staking_delegate_msg = CosmosMsg::Staking(StakingMsg::Delegate {
            validator: anyone.to_string(),
            amount: coin(70000, "ureef"),
        });
        let staking_withdraw_msg =
            CosmosMsg::Distribution(DistributionMsg::WithdrawDelegatorReward {
                validator: anyone.to_string(),
            });

        // owner can send big or small
        let res = query_can_execute(deps.as_ref(), owner.to_string(), send_msg.clone()).unwrap();
        assert!(res.can_execute);
        let res =
            query_can_execute(deps.as_ref(), owner.to_string(), send_msg_large.clone()).unwrap();
        assert!(res.can_execute);
        // owner can stake
        let res = query_can_execute(
            deps.as_ref(),
            owner.to_string(),
            staking_delegate_msg.clone(),
        )
        .unwrap();
        assert!(res.can_execute);

        // spender can send small
        let res = query_can_execute(deps.as_ref(), spender.to_string(), send_msg.clone()).unwrap();
        assert!(res.can_execute);
        // not too big
        let res =
            query_can_execute(deps.as_ref(), spender.to_string(), send_msg_large.clone()).unwrap();
        assert!(!res.can_execute);
        // spender can send staking msgs if permissioned
        let res = query_can_execute(
            deps.as_ref(),
            spender.to_string(),
            staking_delegate_msg.clone(),
        )
        .unwrap();
        assert!(res.can_execute);
        let res = query_can_execute(
            deps.as_ref(),
            spender.to_string(),
            staking_withdraw_msg.clone(),
        )
        .unwrap();
        assert!(!res.can_execute);

        // random person cannot do anything
        let res = query_can_execute(deps.as_ref(), anyone.to_string(), send_msg).unwrap();
        assert!(!res.can_execute);
        let res = query_can_execute(deps.as_ref(), anyone.to_string(), send_msg_large).unwrap();
        assert!(!res.can_execute);
        let res =
            query_can_execute(deps.as_ref(), anyone.to_string(), staking_delegate_msg).unwrap();
        assert!(!res.can_execute);
        let res =
            query_can_execute(deps.as_ref(), anyone.to_string(), staking_withdraw_msg).unwrap();
        assert!(!res.can_execute);
    }
}
