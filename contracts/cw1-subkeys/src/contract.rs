use schemars::JsonSchema;
use std::fmt;

use cosmwasm_std::{
    log, to_binary, Api, BankMsg, Binary, Coin, CosmosMsg, Empty, Env, Extern, HandleResponse,
    HumanAddr, InitResponse, Querier, StdError, StdResult, Storage,
};
use cw1_whitelist::{
    contract::{handle_freeze, handle_update_admins, init as whitelist_init, query_config},
    msg::InitMsg,
    state::config_read,
};
use cw20::Expiration;

use crate::msg::{HandleMsg, QueryMsg};
use crate::state::{allowances, allowances_read, Allowance};
use std::ops::{AddAssign, Sub};

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    whitelist_init(deps, env, msg)
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
    let cfg = config_read(&deps.storage).load()?;
    let owner_raw = &deps.api.canonical_address(&env.message.sender)?;
    // this is the admin behavior (same as cw1-whitelist)
    if cfg.is_admin(owner_raw) {
        let mut res = HandleResponse::default();
        res.messages = msgs;
        res.log = vec![log("action", "execute"), log("owner", env.message.sender)];
        Ok(res)
    } else {
        let mut allowances = allowances(&mut deps.storage);
        let allow = allowances.may_load(owner_raw.as_slice())?;
        let mut allowance =
            allow.ok_or_else(|| StdError::not_found("No allowance for this account"))?;
        for msg in &msgs {
            match msg {
                CosmosMsg::Bank(BankMsg::Send {
                    from_address: _,
                    to_address: _,
                    amount,
                }) => {
                    // Decrease allowance
                    for coin in amount {
                        allowance.balance = allowance.balance.sub(coin.clone())?;
                        // Fails if not enough tokens
                    }
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
    let cfg = config_read(&deps.storage).load()?;
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
    let cfg = config_read(&deps.storage).load()?;
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
        allowance.balance = allowance.balance.sub_saturating(amount.clone())?; // Fails if no tokens
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

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::Allowance { spender } => to_binary(&query_allowance(deps, spender)?),
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

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, MOCK_CONTRACT_ADDR};
    use cosmwasm_std::{coins, BankMsg, StdError, WasmMsg};

    const CANONICAL_LENGTH: usize = 20;

    // you probably want some `setup_test_case` function that inits a contract with 2 admins
    // and 2 subkeys with some allowances. these keys can be constants strings here,
    // used like `HumanAddr::from(admin1)` and then all tests can just run against that

    #[test]
    fn query_allowances() {
        // TODO
        // check the allowances work for accounts with balances and accounts with none
    }

    #[test]
    fn update_admins_and_query() {
        // TODO
        // insure imported logic is wired up properly
    }

    #[test]
    fn increase_allowances() {
        // TODO
        // add to existing account (expires = None) => don't change Expiration (previous should be different than Never)
        // add to existing account (expires = Some)
        // add to new account (expires = None) => default Expiration::Never
        // add to new account (expires = Some)
    }

    #[test]
    fn decrease_allowances() {
        // TODO
        // subtract to existing account (has none of that denom)
        // subtract to existing account (brings denom to 0, other denoms left)
        // subtract to existing account (brings denom to > 0)
        // subtract to existing account (brings denom to 0, no other denoms left => should delete Allowance)
        // subtract from empty account (should error)
    }
}
