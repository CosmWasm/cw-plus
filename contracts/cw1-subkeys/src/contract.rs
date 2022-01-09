use schemars::JsonSchema;
use std::fmt;
use std::ops::{AddAssign, Sub};

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    ensure, ensure_ne, to_binary, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, DistributionMsg,
    Empty, Env, MessageInfo, Order, Response, StakingMsg, StdResult,
};
use cw1::CanExecuteResponse;
use cw1_whitelist::{
    contract::{
        execute_freeze, execute_update_admins, instantiate as whitelist_instantiate,
        query_admin_list,
    },
    msg::InstantiateMsg,
    state::ADMIN_LIST,
};
use cw2::{get_contract_version, set_contract_version};
use cw_storage_plus::Bound;
use cw_utils::Expiration;
use semver::Version;

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
    env: Env,
    info: MessageInfo,
    msgs: Vec<CosmosMsg<T>>,
) -> Result<Response<T>, ContractError>
where
    T: Clone + fmt::Debug + PartialEq + JsonSchema,
{
    let cfg = ADMIN_LIST.load(deps.storage)?;

    // Not an admin - need to check for permissions
    if !cfg.is_admin(info.sender.as_ref()) {
        for msg in &msgs {
            match msg {
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
                        ensure!(
                            !allowance.expires.is_expired(&env.block),
                            ContractError::NoAllowance {}
                        );

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
    let res = Response::new()
        .add_messages(msgs)
        .add_attribute("action", "execute")
        .add_attribute("owner", info.sender);
    Ok(res)
}

pub fn check_staking_permissions(
    staking_msg: &StakingMsg,
    permissions: Permissions,
) -> Result<(), ContractError> {
    match staking_msg {
        StakingMsg::Delegate { .. } => {
            ensure!(permissions.delegate, ContractError::DelegatePerm {});
        }
        StakingMsg::Undelegate { .. } => {
            ensure!(permissions.undelegate, ContractError::UnDelegatePerm {});
        }
        StakingMsg::Redelegate { .. } => {
            ensure!(permissions.redelegate, ContractError::ReDelegatePerm {});
        }
        _ => return Err(ContractError::UnsupportedMessage {}),
    }
    Ok(())
}

pub fn check_distribution_permissions(
    distribution_msg: &DistributionMsg,
    permissions: Permissions,
) -> Result<(), ContractError> {
    match distribution_msg {
        DistributionMsg::SetWithdrawAddress { .. } => {
            ensure!(permissions.withdraw, ContractError::WithdrawAddrPerm {});
        }
        DistributionMsg::WithdrawDelegatorReward { .. } => {
            ensure!(permissions.withdraw, ContractError::WithdrawPerm {});
        }
        _ => return Err(ContractError::UnsupportedMessage {}),
    }
    Ok(())
}

pub fn execute_increase_allowance<T>(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    spender: String,
    amount: Coin,
    expires: Option<Expiration>,
) -> Result<Response<T>, ContractError>
where
    T: Clone + fmt::Debug + PartialEq + JsonSchema,
{
    let cfg = ADMIN_LIST.load(deps.storage)?;
    ensure!(cfg.is_admin(&info.sender), ContractError::Unauthorized {});

    let spender_addr = deps.api.addr_validate(&spender)?;
    ensure_ne!(
        info.sender,
        spender_addr,
        ContractError::CannotSetOwnAccount {}
    );

    ALLOWANCES.update::<_, ContractError>(deps.storage, &spender_addr, |allow| {
        let prev_expires = allow
            .as_ref()
            .map(|allow| allow.expires)
            .unwrap_or_default();

        let mut allowance = allow
            .filter(|allow| !allow.expires.is_expired(&env.block))
            .unwrap_or_default();

        if let Some(exp) = expires {
            if exp.is_expired(&env.block) {
                return Err(ContractError::SettingExpiredAllowance(exp));
            }

            allowance.expires = exp;
        } else if prev_expires.is_expired(&env.block) {
            return Err(ContractError::SettingExpiredAllowance(prev_expires));
        }

        allowance.balance.add_assign(amount.clone());
        Ok(allowance)
    })?;

    let res = Response::new()
        .add_attribute("action", "increase_allowance")
        .add_attribute("owner", info.sender)
        .add_attribute("spender", spender)
        .add_attribute("denomination", amount.denom)
        .add_attribute("amount", amount.amount);
    Ok(res)
}

pub fn execute_decrease_allowance<T>(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    spender: String,
    amount: Coin,
    expires: Option<Expiration>,
) -> Result<Response<T>, ContractError>
where
    T: Clone + fmt::Debug + PartialEq + JsonSchema,
{
    let cfg = ADMIN_LIST.load(deps.storage)?;
    ensure!(cfg.is_admin(&info.sender), ContractError::Unauthorized {});

    let spender_addr = deps.api.addr_validate(&spender)?;
    ensure_ne!(
        info.sender,
        spender_addr,
        ContractError::CannotSetOwnAccount {}
    );

    let allowance =
        ALLOWANCES.update::<_, ContractError>(deps.storage, &spender_addr, |allow| {
            // Fail fast
            let mut allowance = allow
                .filter(|allow| !allow.expires.is_expired(&env.block))
                .ok_or(ContractError::NoAllowance {})?;

            if let Some(exp) = expires {
                if exp.is_expired(&env.block) {
                    return Err(ContractError::SettingExpiredAllowance(exp));
                }

                allowance.expires = exp;
            }

            allowance.balance = allowance.balance.sub_saturating(amount.clone())?; // Tolerates underflows (amount bigger than balance), but fails if there are no tokens at all for the denom (report potential errors)
            Ok(allowance)
        })?;

    if allowance.balance.is_empty() {
        ALLOWANCES.remove(deps.storage, &spender_addr);
    }

    let res = Response::new()
        .add_attribute("action", "decrease_allowance")
        .add_attribute("owner", info.sender)
        .add_attribute("spender", spender)
        .add_attribute("denomination", amount.denom)
        .add_attribute("amount", amount.amount);
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
    ensure!(cfg.is_admin(&info.sender), ContractError::Unauthorized {});

    let spender_addr = deps.api.addr_validate(&spender)?;
    ensure_ne!(
        info.sender,
        spender_addr,
        ContractError::CannotSetOwnAccount {}
    );
    PERMISSIONS.save(deps.storage, &spender_addr, &perm)?;

    let res = Response::new()
        .add_attribute("action", "set_permissions")
        .add_attribute("owner", info.sender)
        .add_attribute("spender", spender)
        .add_attribute("permissions", perm.to_string());
    Ok(res)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::AdminList {} => to_binary(&query_admin_list(deps)?),
        QueryMsg::Allowance { spender } => to_binary(&query_allowance(deps, env, spender)?),
        QueryMsg::Permissions { spender } => to_binary(&query_permissions(deps, spender)?),
        QueryMsg::CanExecute { sender, msg } => {
            to_binary(&query_can_execute(deps, env, sender, msg)?)
        }
        QueryMsg::AllAllowances { start_after, limit } => {
            to_binary(&query_all_allowances(deps, env, start_after, limit)?)
        }
        QueryMsg::AllPermissions { start_after, limit } => {
            to_binary(&query_all_permissions(deps, start_after, limit)?)
        }
    }
}

// if the subkey has no allowance, return an empty struct (not an error)
pub fn query_allowance(deps: Deps, env: Env, spender: String) -> StdResult<Allowance> {
    // we can use unchecked here as it is a query - bad value means a miss, we never write it
    let spender = deps.api.addr_validate(&spender)?;
    let allow = ALLOWANCES
        .may_load(deps.storage, &spender)?
        .filter(|allow| !allow.expires.is_expired(&env.block))
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

fn query_can_execute(
    deps: Deps,
    env: Env,
    sender: String,
    msg: CosmosMsg,
) -> StdResult<CanExecuteResponse> {
    Ok(CanExecuteResponse {
        can_execute: can_execute(deps, env, sender, msg)?,
    })
}

// this can just return booleans and the query_can_execute wrapper creates the struct once, not on every path
fn can_execute(deps: Deps, env: Env, sender: String, msg: CosmosMsg) -> StdResult<bool> {
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
                Some(allow) => {
                    Ok(!allow.expires.is_expired(&env.block) && allow.balance.sub(amount).is_ok())
                }
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
        CosmosMsg::Distribution(distribution_msg) => {
            let perm_opt = PERMISSIONS.may_load(deps.storage, &sender)?;
            match perm_opt {
                Some(permission) => {
                    Ok(check_distribution_permissions(&distribution_msg, permission).is_ok())
                }
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
    env: Env,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<AllAllowancesResponse> {
    let limit = calc_limit(limit);
    // we use raw addresses here....
    let start = start_after.map(|s| Bound::ExclusiveRaw(s.into()));

    let allowances = ALLOWANCES
        .range(deps.storage, start, None, Order::Ascending)
        .filter(|item| {
            if let Ok((_, allow)) = item {
                !allow.expires.is_expired(&env.block)
            } else {
                true
            }
        })
        .take(limit)
        .map(|item| {
            item.map(|(addr, allow)| AllowanceInfo {
                spender: addr.into(),
                balance: allow.balance,
                expires: allow.expires,
            })
        })
        .collect::<StdResult<Vec<_>>>()?;
    Ok(AllAllowancesResponse { allowances })
}

// return a list of all permissions here
pub fn query_all_permissions(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<AllPermissionsResponse> {
    let limit = calc_limit(limit);
    let start = start_after.map(|s| Bound::ExclusiveRaw(s.into()));

    let permissions = PERMISSIONS
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            item.map(|(addr, perm)| PermissionsInfo {
                spender: addr.into(),
                permissions: perm,
            })
        })
        .collect::<StdResult<Vec<_>>>()?;
    Ok(AllPermissionsResponse { permissions })
}

// Migrate contract if version is lower than current version
#[entry_point]
pub fn migrate(deps: DepsMut, _env: Env, _msg: Empty) -> Result<Response, ContractError> {
    let version: Version = CONTRACT_VERSION.parse()?;
    let storage_version: Version = get_contract_version(deps.storage)?.version.parse()?;

    if storage_version < version {
        set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

        // If state structure changed in any contract version in the way migration is needed, it
        // should occur here
    }

    Ok(Response::new())
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::{
        mock_dependencies, mock_env, mock_info, MockApi, MockQuerier, MockStorage,
    };
    use cosmwasm_std::{coin, coins, OwnedDeps, StakingMsg, SubMsg, Timestamp};

    use cw1_whitelist::msg::AdminListResponse;
    use cw2::{get_contract_version, ContractVersion};
    use cw_utils::NativeBalance;

    use crate::state::Permissions;

    use std::collections::HashMap;

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

    // Expiration constant working properly with default `mock_env`
    const NON_EXPIRED_HEIGHT: Expiration = Expiration::AtHeight(22_222);
    const NON_EXPIRED_TIME: Expiration =
        Expiration::AtTime(Timestamp::from_nanos(2_571_797_419_879_305_533));

    const EXPIRED_HEIGHT: Expiration = Expiration::AtHeight(10);
    const EXPIRED_TIME: Expiration = Expiration::AtTime(Timestamp::from_nanos(100));

    /// Helper structure for Suite configuration
    #[derive(Default)]
    struct SuiteConfig {
        spenders: HashMap<&'static str, Spender>,
        admins: Vec<&'static str>,
    }

    impl SuiteConfig {
        fn new() -> Self {
            Self::default()
        }

        fn init(self) -> Suite {
            Suite::init_with_config(self)
        }

        fn with_allowance(mut self, spender: &'static str, allowance: Coin) -> Self {
            self.spenders
                .entry(spender)
                .or_default()
                .allowances
                .push(allowance);
            self
        }

        fn expire_allowances(mut self, spender: &'static str, expires: Expiration) -> Self {
            let item = self.spenders.entry(spender).or_default();
            assert!(
                item.allowances_expire.is_none(),
                "Allowances expiration for spender {} already configured",
                spender
            );
            item.allowances_expire = Some(expires);
            self
        }

        fn with_permissions(mut self, spender: &'static str, permissions: Permissions) -> Self {
            let item = self.spenders.entry(spender).or_default();
            assert!(
                item.permissions.is_none(),
                "Permissions for spender {} already configured",
                spender
            );
            item.permissions = Some(permissions);
            self
        }

        fn with_admin(mut self, admin: &'static str) -> Self {
            self.admins.push(admin);
            self
        }
    }

    #[derive(Default)]
    struct Spender {
        allowances: Vec<Coin>,
        allowances_expire: Option<Expiration>,
        permissions: Option<Permissions>,
    }

    /// Test suite helper unifying test initialization, keeping access to created data
    struct Suite {
        deps: OwnedDeps<MockStorage, MockApi, MockQuerier>,
        owner: MessageInfo,
    }

    impl Suite {
        /// Initializes test case using default config
        fn init() -> Self {
            Self::init_with_config(SuiteConfig::default())
        }

        /// Initialized test case using provided config
        fn init_with_config(config: SuiteConfig) -> Self {
            let mut deps = mock_dependencies();
            let admins = std::iter::once(OWNER)
                .chain(config.admins)
                .map(ToOwned::to_owned)
                .collect();

            let instantiate_msg = InstantiateMsg {
                admins,
                mutable: true,
            };
            let owner = mock_info(OWNER, &[]);

            instantiate(
                deps.as_mut().branch(),
                mock_env(),
                owner.clone(),
                instantiate_msg,
            )
            .unwrap();

            for (name, spender) in config.spenders {
                let Spender {
                    allowances,
                    allowances_expire: expires,
                    permissions,
                } = spender;

                for amount in allowances {
                    let msg = ExecuteMsg::IncreaseAllowance {
                        spender: name.to_owned(),
                        amount,
                        expires,
                    };

                    // Extend block and time, so all alowances are set, even if expired in normal
                    // mock_env
                    let mut env = mock_env();
                    env.block.time = Timestamp::from_nanos(0);
                    env.block.height = 0;
                    execute(deps.as_mut().branch(), env, owner.clone(), msg).unwrap();
                }

                if let Some(permissions) = permissions {
                    let msg = ExecuteMsg::SetPermissions {
                        spender: name.to_owned(),
                        permissions,
                    };
                    execute(deps.as_mut().branch(), mock_env(), owner.clone(), msg).unwrap();
                }
            }

            Self { deps, owner }
        }
    }

    /// Helper function for comparing vectors or another slice-like object as they would represent
    /// set with duplications. Compares sets by first sorting elements using provided ordering.
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

    mod allowance {
        use super::*;

        #[test]
        fn query() {
            let Suite { deps, .. } = SuiteConfig::new()
                .with_allowance(SPENDER1, coin(1, TOKEN))
                .with_allowance(SPENDER2, coin(2, TOKEN))
                .init();

            // Check allowances work for accounts with balances
            let allowance =
                query_allowance(deps.as_ref(), mock_env(), SPENDER1.to_owned()).unwrap();
            assert_eq!(
                allowance,
                Allowance {
                    balance: NativeBalance(vec![coin(1, TOKEN)]),
                    expires: Expiration::Never {},
                }
            );
            let allowance =
                query_allowance(deps.as_ref(), mock_env(), SPENDER2.to_owned()).unwrap();
            assert_eq!(
                allowance,
                Allowance {
                    balance: NativeBalance(vec![coin(2, TOKEN)]),
                    expires: Expiration::Never {},
                }
            );

            // Check allowances work for accounts with no balance
            let allowance =
                query_allowance(deps.as_ref(), mock_env(), SPENDER3.to_string()).unwrap();
            assert_eq!(allowance, Allowance::default());
        }

        #[test]
        fn query_expired() {
            let Suite { deps, .. } = SuiteConfig::new()
                .with_allowance(SPENDER1, coin(1, TOKEN))
                .expire_allowances(SPENDER1, EXPIRED_HEIGHT)
                .init();

            // Check allowances work for accounts with balances
            let allowance =
                query_allowance(deps.as_ref(), mock_env(), SPENDER1.to_owned()).unwrap();
            assert_eq!(
                allowance,
                Allowance {
                    balance: NativeBalance(vec![]),
                    expires: Expiration::Never {},
                }
            );
        }

        #[test]
        fn query_all() {
            let s1_allow = coin(1234, TOKEN);
            let s2_allow = coin(2345, TOKEN);
            let s3_allow = coin(3456, TOKEN);

            let s2_expire = Expiration::Never {};
            let s3_expire = NON_EXPIRED_HEIGHT;

            let Suite { deps, .. } = SuiteConfig::new()
                .with_allowance(SPENDER1, s1_allow.clone())
                .with_allowance(SPENDER2, s2_allow.clone())
                .expire_allowances(SPENDER2, s2_expire)
                .with_allowance(SPENDER3, s3_allow.clone())
                .expire_allowances(SPENDER3, s3_expire)
                // This allowance is already expired - should not occur in result
                .with_allowance(SPENDER4, coin(2222, TOKEN))
                .expire_allowances(SPENDER4, EXPIRED_HEIGHT)
                .init();

            // let's try pagination.
            //
            // Check is tricky, as there is no guarantee about order expiration are received (as it is
            // dependent at least on ordering of insertions), so to check if pagination works, all what
            // can we do is to ensure parts are of expected size, and that collectively all allowances
            // are returned.
            let batch1 = query_all_allowances(deps.as_ref(), mock_env(), None, Some(2))
                .unwrap()
                .allowances;
            assert_eq!(2, batch1.len());

            // now continue from after the last one
            let batch2 = query_all_allowances(
                deps.as_ref(),
                mock_env(),
                Some(batch1[1].spender.clone()),
                Some(2),
            )
            .unwrap()
            .allowances;
            assert_eq!(1, batch2.len());

            let expected = vec![
                AllowanceInfo {
                    spender: SPENDER1.to_owned(),
                    balance: NativeBalance(vec![s1_allow]),
                    expires: Expiration::Never {}, // Not set, expected default
                },
                AllowanceInfo {
                    spender: SPENDER2.to_owned(),
                    balance: NativeBalance(vec![s2_allow]),
                    expires: s2_expire,
                },
                AllowanceInfo {
                    spender: SPENDER3.to_owned(),
                    balance: NativeBalance(vec![s3_allow]),
                    expires: s3_expire,
                },
            ];

            assert_sorted_eq!(
                expected,
                [batch1, batch2].concat(),
                AllowanceInfo::cmp_by_spender
            );
        }
    }

    mod permissions {
        use super::*;

        #[test]
        fn query() {
            let Suite { deps, .. } = SuiteConfig::new()
                .with_permissions(SPENDER1, ALL_PERMS)
                .with_permissions(SPENDER2, NO_PERMS)
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
        fn query_all() {
            let Suite { deps, .. } = SuiteConfig::new()
                .with_permissions(SPENDER1, ALL_PERMS)
                .with_permissions(SPENDER2, NO_PERMS)
                .with_permissions(SPENDER3, NO_PERMS)
                .init();

            // let's try pagination
            let batch1 = query_all_permissions(deps.as_ref(), None, Some(2))
                .unwrap()
                .permissions;
            assert_eq!(batch1.len(), 2);

            let batch2 =
                query_all_permissions(deps.as_ref(), Some(batch1[1].spender.clone()), Some(2))
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
                PermissionsInfo::cmp_by_spender
            );
        }
    }

    mod admins {
        use super::*;

        #[test]
        fn query() {
            let Suite { deps, .. } = SuiteConfig::new().with_admin(ADMIN1).init();

            // Verify
            assert_eq!(
                query_admin_list(deps.as_ref()).unwrap().canonical(),
                AdminListResponse {
                    admins: vec![OWNER.to_owned(), ADMIN1.to_owned()],
                    mutable: true,
                }
                .canonical()
            );
        }

        #[test]
        fn update() {
            let Suite {
                mut deps, owner, ..
            } = SuiteConfig::new().init();

            let rsp = execute(
                deps.as_mut(),
                mock_env(),
                owner,
                ExecuteMsg::UpdateAdmins {
                    admins: vec![OWNER.to_owned(), ADMIN1.to_owned(), ADMIN2.to_owned()],
                },
            )
            .unwrap();

            assert_eq!(rsp.messages, vec![]);
            assert_eq!(rsp.events, vec![]);
            assert_eq!(rsp.data, None);

            assert_eq!(
                query_admin_list(deps.as_ref()).unwrap().canonical(),
                AdminListResponse {
                    admins: vec![OWNER.to_owned(), ADMIN1.to_owned(), ADMIN2.to_owned()],
                    mutable: true,
                }
                .canonical()
            );
        }

        #[test]
        fn non_owner_update() {
            let Suite { mut deps, .. } = SuiteConfig::new().with_admin(ADMIN1).init();
            let info = mock_info(ADMIN1, &[]);

            let rsp = execute(
                deps.as_mut(),
                mock_env(),
                info,
                ExecuteMsg::UpdateAdmins {
                    admins: vec![OWNER.to_owned(), ADMIN1.to_owned(), ADMIN2.to_owned()],
                },
            )
            .unwrap();

            assert_eq!(rsp.messages, vec![]);
            assert_eq!(rsp.events, vec![]);
            assert_eq!(rsp.data, None);

            assert_eq!(
                query_admin_list(deps.as_ref()).unwrap().canonical(),
                AdminListResponse {
                    admins: vec![OWNER.to_owned(), ADMIN1.to_owned(), ADMIN2.to_owned()],
                    mutable: true,
                }
                .canonical()
            );
        }

        #[test]
        fn non_admin_fail_to_update() {
            let Suite { mut deps, .. } = SuiteConfig::new().init();
            let info = mock_info(ADMIN1, &[]);

            execute(
                deps.as_mut(),
                mock_env(),
                info,
                ExecuteMsg::UpdateAdmins {
                    admins: vec![OWNER.to_owned(), ADMIN1.to_owned(), ADMIN2.to_owned()],
                },
            )
            .unwrap_err();

            assert_eq!(
                query_admin_list(deps.as_ref()).unwrap().canonical(),
                AdminListResponse {
                    admins: vec![OWNER.to_owned()],
                    mutable: true,
                }
                .canonical()
            );
        }

        #[test]
        fn removed_owner_fail_to_update() {
            let Suite {
                mut deps, owner, ..
            } = SuiteConfig::new().init();

            // Exact result not checked as it is verified in another test
            execute(
                deps.as_mut(),
                mock_env(),
                owner.clone(),
                ExecuteMsg::UpdateAdmins {
                    admins: vec![ADMIN1.to_owned()],
                },
            )
            .unwrap();

            execute(
                deps.as_mut(),
                mock_env(),
                owner,
                ExecuteMsg::UpdateAdmins {
                    admins: vec![OWNER.to_owned(), ADMIN1.to_owned(), ADMIN2.to_owned()],
                },
            )
            .unwrap_err();

            assert_eq!(
                query_admin_list(deps.as_ref()).unwrap().canonical(),
                AdminListResponse {
                    admins: vec![ADMIN1.to_owned()],
                    mutable: true,
                }
                .canonical()
            );
        }
    }

    mod increase_allowance {
        // TODO: Add cases with increasing expired allowances, and with adding allowance which is
        // already expired

        use super::*;

        #[test]
        fn existing_token() {
            let Suite {
                mut deps, owner, ..
            } = SuiteConfig::new()
                .with_allowance(SPENDER1, coin(1, TOKEN1))
                .init();

            let rsp = execute(
                deps.as_mut(),
                mock_env(),
                owner,
                ExecuteMsg::IncreaseAllowance {
                    spender: SPENDER1.to_owned(),
                    amount: coin(3, TOKEN1),
                    expires: None,
                },
            )
            .unwrap();

            assert_eq!(rsp.messages, vec![]);
            assert_eq!(rsp.events, vec![]);
            assert_eq!(rsp.data, None);

            assert_eq!(
                query_all_allowances(deps.as_ref(), mock_env(), None, None)
                    .unwrap()
                    .canonical(),
                AllAllowancesResponse {
                    allowances: vec![AllowanceInfo {
                        spender: SPENDER1.to_owned(),
                        balance: NativeBalance(vec![coin(4, TOKEN1)]),
                        expires: Expiration::Never {},
                    }]
                }
                .canonical()
            );
        }

        #[test]
        fn with_expiration() {
            let Suite {
                mut deps, owner, ..
            } = SuiteConfig::new()
                .with_allowance(SPENDER1, coin(1, TOKEN1))
                .init();

            let mut env = mock_env();
            env.block.height = 2;

            let rsp = execute(
                deps.as_mut(),
                env,
                owner,
                ExecuteMsg::IncreaseAllowance {
                    spender: SPENDER1.to_owned(),
                    amount: coin(3, TOKEN1),
                    expires: Some(NON_EXPIRED_HEIGHT),
                },
            )
            .unwrap();

            assert_eq!(rsp.messages, vec![]);
            assert_eq!(rsp.events, vec![]);
            assert_eq!(rsp.data, None);

            assert_eq!(
                query_all_allowances(deps.as_ref(), mock_env(), None, None)
                    .unwrap()
                    .canonical(),
                AllAllowancesResponse {
                    allowances: vec![AllowanceInfo {
                        spender: SPENDER1.to_owned(),
                        balance: NativeBalance(vec![coin(4, TOKEN1)]),
                        expires: NON_EXPIRED_HEIGHT,
                    }]
                }
                .canonical()
            );
        }

        #[test]
        fn new_token_on_existing_spender() {
            let Suite {
                mut deps, owner, ..
            } = SuiteConfig::new()
                .with_allowance(SPENDER1, coin(1, TOKEN1))
                .init();

            let rsp = execute(
                deps.as_mut(),
                mock_env(),
                owner,
                ExecuteMsg::IncreaseAllowance {
                    spender: SPENDER1.to_owned(),
                    amount: coin(3, TOKEN2),
                    expires: None,
                },
            )
            .unwrap();

            assert_eq!(rsp.messages, vec![]);
            assert_eq!(rsp.events, vec![]);
            assert_eq!(rsp.data, None);

            assert_eq!(
                query_all_allowances(deps.as_ref(), mock_env(), None, None)
                    .unwrap()
                    .canonical(),
                AllAllowancesResponse {
                    allowances: vec![AllowanceInfo {
                        spender: SPENDER1.to_owned(),
                        balance: NativeBalance(vec![coin(1, TOKEN1), coin(3, TOKEN2)]),
                        expires: Expiration::Never {},
                    }]
                }
                .canonical()
            );
        }

        #[test]
        fn new_spender() {
            let Suite {
                mut deps, owner, ..
            } = SuiteConfig::new()
                .with_allowance(SPENDER1, coin(1, TOKEN1))
                .init();

            let rsp = execute(
                deps.as_mut(),
                mock_env(),
                owner,
                ExecuteMsg::IncreaseAllowance {
                    spender: SPENDER2.to_owned(),
                    amount: coin(3, TOKEN1),
                    expires: None,
                },
            )
            .unwrap();

            assert_eq!(rsp.messages, vec![]);
            assert_eq!(rsp.events, vec![]);
            assert_eq!(rsp.data, None);

            assert_eq!(
                query_all_allowances(deps.as_ref(), mock_env(), None, None)
                    .unwrap()
                    .canonical(),
                AllAllowancesResponse {
                    allowances: vec![
                        AllowanceInfo {
                            spender: SPENDER1.to_owned(),
                            balance: NativeBalance(vec![coin(1, TOKEN1)]),
                            expires: Expiration::Never {},
                        },
                        AllowanceInfo {
                            spender: SPENDER2.to_owned(),
                            balance: NativeBalance(vec![coin(3, TOKEN1)]),
                            expires: Expiration::Never {},
                        }
                    ]
                }
                .canonical(),
            );
        }

        #[test]
        fn new_spender_with_expiration() {
            let Suite {
                mut deps, owner, ..
            } = SuiteConfig::new()
                .with_allowance(SPENDER1, coin(1, TOKEN1))
                .init();

            let rsp = execute(
                deps.as_mut(),
                mock_env(),
                owner,
                ExecuteMsg::IncreaseAllowance {
                    spender: SPENDER2.to_owned(),
                    amount: coin(3, TOKEN1),
                    expires: Some(NON_EXPIRED_HEIGHT),
                },
            )
            .unwrap();

            assert_eq!(rsp.messages, vec![]);
            assert_eq!(rsp.events, vec![]);
            assert_eq!(rsp.data, None);

            assert_eq!(
                query_all_allowances(deps.as_ref(), mock_env(), None, None)
                    .unwrap()
                    .canonical(),
                AllAllowancesResponse {
                    allowances: vec![
                        AllowanceInfo {
                            spender: SPENDER1.to_owned(),
                            balance: NativeBalance(vec![coin(1, TOKEN1)]),
                            expires: Expiration::Never {},
                        },
                        AllowanceInfo {
                            spender: SPENDER2.to_owned(),
                            balance: NativeBalance(vec![coin(3, TOKEN1)]),
                            expires: NON_EXPIRED_HEIGHT,
                        }
                    ]
                }
                .canonical(),
            );
        }

        #[test]
        fn previous_expired() {
            let Suite {
                mut deps, owner, ..
            } = SuiteConfig::new()
                .with_allowance(SPENDER1, coin(1, TOKEN1))
                .expire_allowances(SPENDER1, EXPIRED_HEIGHT)
                .init();

            let rsp = execute(
                deps.as_mut(),
                mock_env(),
                owner,
                ExecuteMsg::IncreaseAllowance {
                    spender: SPENDER1.to_owned(),
                    amount: coin(2, TOKEN2),
                    expires: Some(NON_EXPIRED_TIME),
                },
            )
            .unwrap();

            assert_eq!(rsp.messages, vec![]);
            assert_eq!(rsp.events, vec![]);
            assert_eq!(rsp.data, None);

            assert_eq!(
                query_all_allowances(deps.as_ref(), mock_env(), None, None)
                    .unwrap()
                    .canonical(),
                AllAllowancesResponse {
                    allowances: vec![AllowanceInfo {
                        spender: SPENDER1.to_owned(),
                        balance: NativeBalance(vec![coin(2, TOKEN2)]),
                        expires: NON_EXPIRED_TIME,
                    }]
                }
                .canonical(),
            );
        }

        #[test]
        fn set_expired() {
            let Suite {
                mut deps, owner, ..
            } = SuiteConfig::new()
                .with_allowance(SPENDER1, coin(1, TOKEN1))
                .expire_allowances(SPENDER1, NON_EXPIRED_HEIGHT)
                .init();

            let rsp = execute(
                deps.as_mut(),
                mock_env(),
                owner,
                ExecuteMsg::IncreaseAllowance {
                    spender: SPENDER1.to_owned(),
                    amount: coin(2, TOKEN2),
                    expires: Some(EXPIRED_TIME),
                },
            );
            assert_eq!(
                rsp,
                Err(ContractError::SettingExpiredAllowance(EXPIRED_TIME))
            );

            assert_eq!(
                query_all_allowances(deps.as_ref(), mock_env(), None, None)
                    .unwrap()
                    .canonical(),
                AllAllowancesResponse {
                    allowances: vec![AllowanceInfo {
                        spender: SPENDER1.to_owned(),
                        balance: NativeBalance(vec![coin(1, TOKEN1)]),
                        expires: NON_EXPIRED_HEIGHT,
                    }]
                }
                .canonical(),
            );
        }

        #[test]
        fn update_expired_with_no_expiration() {
            let Suite {
                mut deps, owner, ..
            } = SuiteConfig::new()
                .with_allowance(SPENDER1, coin(1, TOKEN1))
                .expire_allowances(SPENDER1, EXPIRED_HEIGHT)
                .init();

            execute(
                deps.as_mut(),
                mock_env(),
                owner,
                ExecuteMsg::IncreaseAllowance {
                    spender: SPENDER1.to_owned(),
                    amount: coin(2, TOKEN2),
                    expires: None,
                },
            )
            .unwrap_err();

            assert_eq!(
                query_all_allowances(deps.as_ref(), mock_env(), None, None)
                    .unwrap()
                    .canonical(),
                AllAllowancesResponse { allowances: vec![] }.canonical(),
            );
        }
    }

    mod decrease_allowances {
        // TODO: Add cases with decreasing expired allowances, and with adding allowance which is
        // already expired

        use super::*;

        #[test]
        fn existing_token_partial() {
            let Suite {
                mut deps, owner, ..
            } = SuiteConfig::new()
                .with_allowance(SPENDER1, coin(10, TOKEN1))
                .expire_allowances(SPENDER1, NON_EXPIRED_HEIGHT)
                .init();

            let rsp = execute(
                deps.as_mut(),
                mock_env(),
                owner,
                ExecuteMsg::DecreaseAllowance {
                    spender: SPENDER1.to_owned(),
                    amount: coin(4, TOKEN1),
                    expires: None,
                },
            )
            .unwrap();

            assert_eq!(rsp.messages, vec![]);
            assert_eq!(rsp.events, vec![]);
            assert_eq!(rsp.data, None);

            assert_eq!(
                query_all_allowances(deps.as_ref(), mock_env(), None, None)
                    .unwrap()
                    .canonical(),
                AllAllowancesResponse {
                    allowances: vec![AllowanceInfo {
                        spender: SPENDER1.to_owned(),
                        balance: NativeBalance(vec![coin(6, TOKEN1)]),
                        expires: NON_EXPIRED_HEIGHT,
                    }]
                }
                .canonical()
            );
        }

        #[test]
        fn existing_token_whole() {
            let Suite {
                mut deps, owner, ..
            } = SuiteConfig::new()
                .with_allowance(SPENDER1, coin(10, TOKEN1))
                .with_allowance(SPENDER1, coin(20, TOKEN2))
                .expire_allowances(SPENDER1, NON_EXPIRED_HEIGHT)
                .init();

            let rsp = execute(
                deps.as_mut(),
                mock_env(),
                owner,
                ExecuteMsg::DecreaseAllowance {
                    spender: SPENDER1.to_owned(),
                    amount: coin(10, TOKEN1),
                    expires: None,
                },
            )
            .unwrap();

            assert_eq!(rsp.messages, vec![]);
            assert_eq!(rsp.events, vec![]);
            assert_eq!(rsp.data, None);

            assert_eq!(
                query_all_allowances(deps.as_ref(), mock_env(), None, None)
                    .unwrap()
                    .canonical(),
                AllAllowancesResponse {
                    allowances: vec![AllowanceInfo {
                        spender: SPENDER1.to_owned(),
                        balance: NativeBalance(vec![coin(20, TOKEN2)]),
                        expires: NON_EXPIRED_HEIGHT,
                    }]
                }
                .canonical()
            );
        }

        #[test]
        fn existing_token_underflow() {
            let Suite {
                mut deps, owner, ..
            } = SuiteConfig::new()
                .with_allowance(SPENDER1, coin(10, TOKEN1))
                .with_allowance(SPENDER1, coin(20, TOKEN2))
                .expire_allowances(SPENDER1, NON_EXPIRED_HEIGHT)
                .init();

            let rsp = execute(
                deps.as_mut(),
                mock_env(),
                owner,
                ExecuteMsg::DecreaseAllowance {
                    spender: SPENDER1.to_owned(),
                    amount: coin(15, TOKEN1),
                    expires: None,
                },
            )
            .unwrap();

            assert_eq!(rsp.messages, vec![]);
            assert_eq!(rsp.events, vec![]);
            assert_eq!(rsp.data, None);

            assert_eq!(
                query_all_allowances(deps.as_ref(), mock_env(), None, None)
                    .unwrap()
                    .canonical(),
                AllAllowancesResponse {
                    allowances: vec![AllowanceInfo {
                        spender: SPENDER1.to_owned(),
                        balance: NativeBalance(vec![coin(20, TOKEN2)]),
                        expires: NON_EXPIRED_HEIGHT,
                    }]
                }
                .canonical()
            );
        }

        #[test]
        fn last_existing_token_whole() {
            let Suite {
                mut deps, owner, ..
            } = SuiteConfig::new()
                .with_allowance(SPENDER1, coin(10, TOKEN1))
                .expire_allowances(SPENDER1, NON_EXPIRED_HEIGHT)
                .init();

            let rsp = execute(
                deps.as_mut(),
                mock_env(),
                owner,
                ExecuteMsg::DecreaseAllowance {
                    spender: SPENDER1.to_owned(),
                    amount: coin(10, TOKEN1),
                    expires: None,
                },
            )
            .unwrap();

            assert_eq!(rsp.messages, vec![]);
            assert_eq!(rsp.events, vec![]);
            assert_eq!(rsp.data, None);

            assert_eq!(
                query_all_allowances(deps.as_ref(), mock_env(), None, None)
                    .unwrap()
                    .canonical(),
                AllAllowancesResponse { allowances: vec![] }.canonical()
            );
        }

        #[test]
        fn with_expiration() {
            let Suite {
                mut deps, owner, ..
            } = SuiteConfig::new()
                .with_allowance(SPENDER1, coin(10, TOKEN1))
                .expire_allowances(SPENDER1, NON_EXPIRED_HEIGHT)
                .init();

            let rsp = execute(
                deps.as_mut(),
                mock_env(),
                owner,
                ExecuteMsg::DecreaseAllowance {
                    spender: SPENDER1.to_owned(),
                    amount: coin(4, TOKEN1),
                    expires: Some(NON_EXPIRED_TIME),
                },
            )
            .unwrap();

            assert_eq!(rsp.messages, vec![]);
            assert_eq!(rsp.events, vec![]);
            assert_eq!(rsp.data, None);

            assert_eq!(
                query_all_allowances(deps.as_ref(), mock_env(), None, None)
                    .unwrap()
                    .canonical(),
                AllAllowancesResponse {
                    allowances: vec![AllowanceInfo {
                        spender: SPENDER1.to_owned(),
                        balance: NativeBalance(vec![coin(6, TOKEN1)]),
                        expires: NON_EXPIRED_TIME,
                    }]
                }
                .canonical()
            );
        }

        #[test]
        fn non_exisiting_token() {
            let Suite {
                mut deps, owner, ..
            } = SuiteConfig::new()
                .with_allowance(SPENDER1, coin(10, TOKEN1))
                .expire_allowances(SPENDER1, NON_EXPIRED_HEIGHT)
                .init();

            execute(
                deps.as_mut(),
                mock_env(),
                owner,
                ExecuteMsg::DecreaseAllowance {
                    spender: SPENDER1.to_owned(),
                    amount: coin(4, TOKEN2),
                    expires: None,
                },
            )
            .unwrap_err();

            assert_eq!(
                query_all_allowances(deps.as_ref(), mock_env(), None, None)
                    .unwrap()
                    .canonical(),
                AllAllowancesResponse {
                    allowances: vec![AllowanceInfo {
                        spender: SPENDER1.to_owned(),
                        balance: NativeBalance(vec![coin(10, TOKEN1)]),
                        expires: NON_EXPIRED_HEIGHT,
                    }]
                }
                .canonical()
            );
        }

        #[test]
        fn non_exisiting_spender() {
            let Suite {
                mut deps, owner, ..
            } = SuiteConfig::new()
                .with_allowance(SPENDER1, coin(10, TOKEN1))
                .init();

            execute(
                deps.as_mut(),
                mock_env(),
                owner,
                ExecuteMsg::DecreaseAllowance {
                    spender: SPENDER2.to_owned(),
                    amount: coin(4, TOKEN1),
                    expires: None,
                },
            )
            .unwrap_err();

            assert_eq!(
                query_all_allowances(deps.as_ref(), mock_env(), None, None)
                    .unwrap()
                    .canonical(),
                AllAllowancesResponse {
                    allowances: vec![AllowanceInfo {
                        spender: SPENDER1.to_owned(),
                        balance: NativeBalance(vec![coin(10, TOKEN1)]),
                        expires: Expiration::Never {},
                    }]
                }
                .canonical()
            );
        }

        #[test]
        fn previous_expired() {
            let Suite {
                mut deps, owner, ..
            } = SuiteConfig::new()
                .with_allowance(SPENDER1, coin(3, TOKEN1))
                .expire_allowances(SPENDER1, EXPIRED_HEIGHT)
                .init();

            execute(
                deps.as_mut(),
                mock_env(),
                owner,
                ExecuteMsg::DecreaseAllowance {
                    spender: SPENDER1.to_owned(),
                    amount: coin(2, TOKEN1),
                    expires: Some(NON_EXPIRED_TIME),
                },
            )
            .unwrap_err();

            assert_eq!(
                query_all_allowances(deps.as_ref(), mock_env(), None, None)
                    .unwrap()
                    .canonical(),
                AllAllowancesResponse { allowances: vec![] }.canonical(),
            );
        }

        #[test]
        fn set_expired() {
            let Suite {
                mut deps, owner, ..
            } = SuiteConfig::new()
                .with_allowance(SPENDER1, coin(3, TOKEN1))
                .expire_allowances(SPENDER1, NON_EXPIRED_HEIGHT)
                .init();

            execute(
                deps.as_mut(),
                mock_env(),
                owner,
                ExecuteMsg::IncreaseAllowance {
                    spender: SPENDER1.to_owned(),
                    amount: coin(2, TOKEN1),
                    expires: Some(EXPIRED_TIME),
                },
            )
            .unwrap_err();

            assert_eq!(
                query_all_allowances(deps.as_ref(), mock_env(), None, None)
                    .unwrap()
                    .canonical(),
                AllAllowancesResponse {
                    allowances: vec![AllowanceInfo {
                        spender: SPENDER1.to_owned(),
                        balance: NativeBalance(vec![coin(3, TOKEN1)]),
                        expires: NON_EXPIRED_HEIGHT,
                    }]
                }
                .canonical(),
            );
        }
    }

    mod spend {
        use super::*;

        #[test]
        fn with_allowance() {
            let Suite { mut deps, .. } = SuiteConfig::new()
                .with_allowance(SPENDER1, coin(10, TOKEN1))
                .init();

            let msgs = vec![BankMsg::Send {
                to_address: SPENDER2.to_owned(),
                amount: coins(6, TOKEN1),
            }
            .into()];

            let info = mock_info(SPENDER1, &[]);

            let rsp = execute(
                deps.as_mut(),
                mock_env(),
                info,
                ExecuteMsg::Execute { msgs: msgs.clone() },
            )
            .unwrap();

            assert_eq!(
                rsp.messages,
                msgs.into_iter().map(SubMsg::new).collect::<Vec<_>>()
            );
            assert_eq!(rsp.events, vec![]);
            assert_eq!(rsp.data, None);

            assert_eq!(
                query_all_allowances(deps.as_ref(), mock_env(), None, None)
                    .unwrap()
                    .canonical(),
                AllAllowancesResponse {
                    allowances: vec![AllowanceInfo {
                        spender: SPENDER1.to_owned(),
                        balance: NativeBalance(vec![coin(4, TOKEN1)]),
                        expires: Expiration::Never {},
                    }]
                }
                .canonical()
            );
        }

        #[test]
        fn without_allowance() {
            let Suite { mut deps, .. } = Suite::init();

            let msgs = vec![BankMsg::Send {
                to_address: SPENDER2.to_owned(),
                amount: coins(6, TOKEN1),
            }
            .into()];

            let info = mock_info(SPENDER1, &[]);

            execute(
                deps.as_mut(),
                mock_env(),
                info,
                ExecuteMsg::Execute { msgs },
            )
            .unwrap_err();

            assert_eq!(
                query_all_allowances(deps.as_ref(), mock_env(), None, None)
                    .unwrap()
                    .canonical(),
                AllAllowancesResponse { allowances: vec![] }.canonical()
            );
        }

        #[test]
        fn not_enough_allowance() {
            let Suite { mut deps, .. } = SuiteConfig::new()
                .with_allowance(SPENDER1, coin(10, TOKEN1))
                .init();

            let msgs = vec![BankMsg::Send {
                to_address: SPENDER2.to_owned(),
                amount: coins(20, TOKEN1),
            }
            .into()];

            let info = mock_info(SPENDER1, &[]);

            execute(
                deps.as_mut(),
                mock_env(),
                info,
                ExecuteMsg::Execute { msgs },
            )
            .unwrap_err();

            assert_eq!(
                query_all_allowances(deps.as_ref(), mock_env(), None, None)
                    .unwrap()
                    .canonical(),
                AllAllowancesResponse {
                    allowances: vec![AllowanceInfo {
                        spender: SPENDER1.to_owned(),
                        balance: NativeBalance(coins(10, TOKEN1)),
                        expires: Expiration::Never {}
                    }]
                }
                .canonical()
            );
        }

        #[test]
        fn time_allowance_expired() {
            let Suite { mut deps, .. } = SuiteConfig::new()
                .with_allowance(SPENDER1, coin(10, TOKEN1))
                .expire_allowances(SPENDER1, EXPIRED_TIME)
                .init();

            let msgs = vec![BankMsg::Send {
                to_address: SPENDER2.to_owned(),
                amount: coins(6, TOKEN1),
            }
            .into()];

            let info = mock_info(SPENDER1, &[]);
            execute(
                deps.as_mut(),
                mock_env(),
                info,
                ExecuteMsg::Execute { msgs },
            )
            .unwrap_err();

            assert_eq!(
                query_all_allowances(deps.as_ref(), mock_env(), None, None)
                    .unwrap()
                    .canonical(),
                AllAllowancesResponse { allowances: vec![] }.canonical()
            );
        }

        #[test]
        fn height_allowance_expired() {
            let Suite { mut deps, .. } = SuiteConfig::new()
                .with_allowance(SPENDER1, coin(10, TOKEN1))
                .expire_allowances(SPENDER1, EXPIRED_HEIGHT)
                .init();

            let msgs = vec![BankMsg::Send {
                to_address: SPENDER2.to_owned(),
                amount: coins(6, TOKEN1),
            }
            .into()];

            let info = mock_info(SPENDER1, &[]);
            execute(
                deps.as_mut(),
                mock_env(),
                info,
                ExecuteMsg::Execute { msgs },
            )
            .unwrap_err();

            assert_eq!(
                query_all_allowances(deps.as_ref(), mock_env(), None, None)
                    .unwrap()
                    .canonical(),
                AllAllowancesResponse { allowances: vec![] }.canonical()
            );
        }

        #[test]
        fn admin_without_allowance() {
            let Suite { mut deps, .. } = SuiteConfig::new().with_admin(ADMIN1).init();

            let msgs = vec![BankMsg::Send {
                to_address: SPENDER2.to_owned(),
                amount: coins(20, TOKEN1),
            }
            .into()];

            let info = mock_info(ADMIN1, &[]);

            let rsp = execute(
                deps.as_mut(),
                mock_env(),
                info,
                ExecuteMsg::Execute { msgs: msgs.clone() },
            )
            .unwrap();

            assert_eq!(
                rsp.messages,
                msgs.into_iter().map(SubMsg::new).collect::<Vec<_>>()
            );
            assert_eq!(rsp.events, vec![]);
            assert_eq!(rsp.data, None);

            assert_eq!(
                query_all_allowances(deps.as_ref(), mock_env(), None, None)
                    .unwrap()
                    .canonical(),
                AllAllowancesResponse { allowances: vec![] }.canonical()
            );
        }
    }

    mod custom_msg {
        use super::*;

        #[test]
        fn admin() {
            let Suite { mut deps, .. } = SuiteConfig::new().with_admin(ADMIN1).init();

            let info = mock_info(ADMIN1, &[]);

            let msgs = vec![CosmosMsg::Custom(Empty {})];

            let rsp = execute(
                deps.as_mut(),
                mock_env(),
                info,
                ExecuteMsg::Execute { msgs: msgs.clone() },
            )
            .unwrap();

            assert_eq!(
                rsp.messages,
                msgs.into_iter().map(SubMsg::new).collect::<Vec<_>>()
            );
            assert_eq!(rsp.events, vec![]);
            assert_eq!(rsp.data, None);
        }

        #[test]
        fn non_admin() {
            let Suite { mut deps, .. } = SuiteConfig::new().with_admin(ADMIN1).init();

            let info = mock_info(SPENDER1, &[]);

            let msgs = vec![CosmosMsg::Custom(Empty {})];

            let _rsp = execute(
                deps.as_mut(),
                mock_env(),
                info,
                ExecuteMsg::Execute { msgs },
            )
            .unwrap_err();
        }
    }

    mod staking_permission {
        use super::*;

        #[test]
        fn allowed() {
            let Suite { mut deps, .. } = SuiteConfig::new()
                .with_permissions(SPENDER1, ALL_PERMS)
                .init();

            let msgs = vec![
                StakingMsg::Delegate {
                    validator: "validator1".to_owned(),
                    amount: coin(10, TOKEN1),
                }
                .into(),
                StakingMsg::Redelegate {
                    src_validator: "validator1".to_owned(),
                    dst_validator: "validator2".to_owned(),
                    amount: coin(15, TOKEN1),
                }
                .into(),
                StakingMsg::Undelegate {
                    validator: "validator1".to_owned(),
                    amount: coin(10, TOKEN1),
                }
                .into(),
                DistributionMsg::WithdrawDelegatorReward {
                    validator: "validator1".to_owned(),
                }
                .into(),
            ];

            for msg in msgs {
                let msgs = vec![msg];
                let rsp = execute(
                    deps.as_mut(),
                    mock_env(),
                    mock_info(SPENDER1, &[]),
                    ExecuteMsg::Execute { msgs: msgs.clone() },
                )
                .unwrap();

                assert_eq!(
                    rsp.messages,
                    msgs.into_iter().map(SubMsg::new).collect::<Vec<_>>()
                );
                assert_eq!(rsp.events, vec![]);
                assert_eq!(rsp.data, None);
            }
        }

        #[test]
        fn admin() {
            let Suite { mut deps, .. } = SuiteConfig::new().with_admin(ADMIN1).init();

            let msgs = vec![
                StakingMsg::Delegate {
                    validator: "validator1".to_owned(),
                    amount: coin(10, TOKEN1),
                }
                .into(),
                StakingMsg::Redelegate {
                    src_validator: "validator1".to_owned(),
                    dst_validator: "validator2".to_owned(),
                    amount: coin(15, TOKEN1),
                }
                .into(),
                StakingMsg::Undelegate {
                    validator: "validator1".to_owned(),
                    amount: coin(10, TOKEN1),
                }
                .into(),
                DistributionMsg::WithdrawDelegatorReward {
                    validator: "validator1".to_owned(),
                }
                .into(),
            ];

            for msg in msgs {
                let msgs = vec![msg];
                let rsp = execute(
                    deps.as_mut(),
                    mock_env(),
                    mock_info(ADMIN1, &[]),
                    ExecuteMsg::Execute { msgs: msgs.clone() },
                )
                .unwrap();

                assert_eq!(
                    rsp.messages,
                    msgs.into_iter().map(SubMsg::new).collect::<Vec<_>>()
                );
                assert_eq!(rsp.events, vec![]);
                assert_eq!(rsp.data, None);
            }
        }

        #[test]
        fn reject() {
            let Suite { mut deps, .. } = Suite::init();

            let msgs = vec![
                StakingMsg::Delegate {
                    validator: "validator1".to_owned(),
                    amount: coin(10, TOKEN1),
                }
                .into(),
                StakingMsg::Redelegate {
                    src_validator: "validator1".to_owned(),
                    dst_validator: "validator2".to_owned(),
                    amount: coin(15, TOKEN1),
                }
                .into(),
                StakingMsg::Undelegate {
                    validator: "validator1".to_owned(),
                    amount: coin(10, TOKEN1),
                }
                .into(),
                DistributionMsg::WithdrawDelegatorReward {
                    validator: "validator1".to_owned(),
                }
                .into(),
            ];

            for msg in msgs {
                let msgs = vec![msg];
                execute(
                    deps.as_mut(),
                    mock_env(),
                    mock_info(SPENDER1, &[]),
                    ExecuteMsg::Execute { msgs },
                )
                .unwrap_err();
            }
        }
    }

    mod can_execute {
        use super::*;

        #[test]
        fn allowed() {
            let Suite { deps, .. } = SuiteConfig::new()
                .with_permissions(SPENDER1, ALL_PERMS)
                .with_allowance(SPENDER1, coin(10, TOKEN1))
                .init();

            let msgs: Vec<CosmosMsg> = vec![
                BankMsg::Send {
                    to_address: SPENDER2.to_owned(),
                    amount: coins(5, TOKEN1),
                }
                .into(),
                StakingMsg::Delegate {
                    validator: SPENDER2.to_owned(),
                    amount: coin(8, TOKEN),
                }
                .into(),
                DistributionMsg::WithdrawDelegatorReward {
                    validator: SPENDER2.to_owned(),
                }
                .into(),
            ];

            for msg in msgs {
                let resp =
                    query_can_execute(deps.as_ref(), mock_env(), SPENDER1.to_owned(), msg.clone())
                        .unwrap();

                assert_eq!(
                    resp,
                    CanExecuteResponse { can_execute: true },
                    "Original message: {:#?}",
                    msg
                );
            }
        }

        #[test]
        fn not_enough_allowance() {
            let Suite { deps, .. } = SuiteConfig::new()
                .with_allowance(SPENDER1, coin(10, TOKEN1))
                .init();

            let msg: CosmosMsg = BankMsg::Send {
                to_address: SPENDER2.to_owned(),
                amount: coins(16, TOKEN1),
            }
            .into();

            let resp =
                query_can_execute(deps.as_ref(), mock_env(), SPENDER1.to_owned(), msg).unwrap();

            assert_eq!(resp, CanExecuteResponse { can_execute: false });
        }

        #[test]
        fn expired_allowance() {
            let Suite { deps, .. } = SuiteConfig::new()
                .with_allowance(SPENDER1, coin(10, TOKEN1))
                .expire_allowances(SPENDER1, EXPIRED_TIME)
                .init();

            let msg: CosmosMsg = BankMsg::Send {
                to_address: SPENDER2.to_owned(),
                amount: coins(5, TOKEN1),
            }
            .into();

            let resp =
                query_can_execute(deps.as_ref(), mock_env(), SPENDER1.to_owned(), msg).unwrap();

            assert_eq!(resp, CanExecuteResponse { can_execute: false });
        }

        #[test]
        fn missing_permissions() {
            let Suite { deps, .. } = SuiteConfig::new()
                .with_permissions(SPENDER1, NO_PERMS)
                .init();

            let msgs: Vec<CosmosMsg> = vec![
                StakingMsg::Delegate {
                    validator: SPENDER2.to_owned(),
                    amount: coin(8, TOKEN),
                }
                .into(),
                DistributionMsg::WithdrawDelegatorReward {
                    validator: SPENDER2.to_owned(),
                }
                .into(),
            ];

            for msg in msgs {
                let resp =
                    query_can_execute(deps.as_ref(), mock_env(), SPENDER1.to_owned(), msg.clone())
                        .unwrap();

                assert_eq!(
                    resp,
                    CanExecuteResponse { can_execute: false },
                    "Original message: {:#?}",
                    msg
                );
            }
        }

        #[test]
        fn custom() {
            let Suite { deps, .. } = SuiteConfig::new()
                .with_permissions(SPENDER1, ALL_PERMS)
                .init();

            let msg: CosmosMsg = CosmosMsg::Custom(Empty {});

            let resp =
                query_can_execute(deps.as_ref(), mock_env(), SPENDER1.to_owned(), msg).unwrap();

            assert_eq!(resp, CanExecuteResponse { can_execute: false });
        }

        #[test]
        fn admin() {
            let Suite { deps, .. } = SuiteConfig::new().with_admin(ADMIN1).init();

            let msgs = vec![
                BankMsg::Send {
                    to_address: SPENDER2.to_owned(),
                    amount: coins(5, TOKEN1),
                }
                .into(),
                StakingMsg::Delegate {
                    validator: SPENDER2.to_owned(),
                    amount: coin(8, TOKEN),
                }
                .into(),
                DistributionMsg::WithdrawDelegatorReward {
                    validator: SPENDER2.to_owned(),
                }
                .into(),
                CosmosMsg::Custom(Empty {}),
            ];

            for msg in msgs {
                let resp =
                    query_can_execute(deps.as_ref(), mock_env(), ADMIN1.to_owned(), msg.clone())
                        .unwrap();

                assert_eq!(
                    resp,
                    CanExecuteResponse { can_execute: true },
                    "Original message: {:#?}",
                    msg
                );
            }
        }
    }

    // tests permissions and allowances are independent features and does not affect each other
    #[test]
    fn permissions_allowances_independent() {
        let mut deps = mock_dependencies();

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
        let res_allow = query_allowance(deps.as_ref(), mock_env(), spender1.to_string()).unwrap();
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
        let res_allow = query_allowance(deps.as_ref(), mock_env(), spender2.to_string()).unwrap();
        assert_eq!(allow, res_allow);
    }
}
