use crate::state::{read_config, read_state, store_state, Config, State};

use crate::math::decimal_summation_in_256;

use cosmwasm_std::{
    attr, CosmosMsg, Decimal, DepsMut, Env, MessageInfo, Response, StdError, StdResult,
};
use terra_cosmwasm::{create_swap_msg, ExchangeRatesResponse, TerraMsgWrapper, TerraQuerier};
/// Swap all native tokens to reward_denom
/// Only hub_contract is allowed to execute
#[allow(clippy::if_same_then_else)]
pub fn execute_swap(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> StdResult<Response<TerraMsgWrapper>> {
    let config = read_config(deps.storage)?;
    let sender_raw = deps.api.addr_canonicalize(info.sender.as_str())?;

    if sender_raw != config.hub_contract {
        return Err(StdError::generic_err("unauthorized"));
    }

    let contr_addr = env.contract.address;
    let balance = deps.querier.query_all_balances(contr_addr)?;
    let mut messages: Vec<CosmosMsg<TerraMsgWrapper>> = Vec::new();

    let reward_denom = config.reward_denom;

    let mut is_listed = true;

    let denoms: Vec<String> = balance.iter().map(|item| item.denom.clone()).collect();

    if query_exchange_rates(&deps, reward_denom.clone(), denoms).is_err() {
        is_listed = false;
    }

    for coin in balance {
        if coin.denom == reward_denom.clone() {
            continue;
        }
        if is_listed {
            messages.push(create_swap_msg(coin, reward_denom.to_string()));
        } else if query_exchange_rates(&deps, reward_denom.clone(), vec![coin.denom.clone()])
            .is_ok()
        {
            messages.push(create_swap_msg(coin, reward_denom.to_string()));
        }
    }

    let res = Response::new()
        .add_messages(messages)
        .add_attributes(vec![attr("action", "swap")]);

    Ok(res)
}

/// Increase global_index according to claimed rewards amount
/// Only hub_contract is allowed to execute
pub fn execute_update_global_index(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> StdResult<Response<TerraMsgWrapper>> {
    let config: Config = read_config(deps.storage)?;
    let mut state: State = read_state(deps.storage)?;

    // Permission check
    if config.hub_contract != deps.api.addr_canonicalize(info.sender.as_str())? {
        return Err(StdError::generic_err("Unauthorized"));
    }

    // Zero staking balance check
    if state.total_balance.is_zero() {
        return Err(StdError::generic_err("No asset is bonded by Hub"));
    }

    let reward_denom = read_config(deps.storage)?.reward_denom;

    // Load the reward contract balance
    let balance = deps
        .querier
        .query_balance(env.contract.address, reward_denom.as_str())?;

    let previous_balance = state.prev_reward_balance;

    // claimed_rewards = current_balance - prev_balance;
    let claimed_rewards = balance.amount.checked_sub(previous_balance)?;

    state.prev_reward_balance = balance.amount;

    // global_index += claimed_rewards / total_balance;
    state.global_index = decimal_summation_in_256(
        state.global_index,
        Decimal::from_ratio(claimed_rewards, state.total_balance),
    );
    store_state(deps.storage, &state)?;

    let attributes = vec![
        attr("action", "update_global_index"),
        attr("claimed_rewards", claimed_rewards),
    ];
    let res = Response::new().add_attributes(attributes);

    Ok(res)
}

pub fn query_exchange_rates(
    deps: &DepsMut,
    base_denom: String,
    quote_denoms: Vec<String>,
) -> StdResult<ExchangeRatesResponse> {
    let querier = TerraQuerier::new(&deps.querier);
    let res: ExchangeRatesResponse = querier.query_exchange_rates(base_denom, quote_denoms)?;
    Ok(res)
}
