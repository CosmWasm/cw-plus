use cosmwasm_std::{
    log, Api, Binary, BlockInfo, CanonicalAddr, Env, Extern, HandleResponse, HumanAddr, Querier,
    StdError, StdResult, Storage, Uint128,
};
use cw20::{AllowanceResponse, Cw20ReceiveMsg, Expiration};

use crate::state::{allowance_remove, allowances, allowances_read, balances, meta};

pub fn handle_increase_allowance<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    spender: HumanAddr,
    amount: Uint128,
    expires: Option<Expiration>,
) -> StdResult<HandleResponse> {
    let spender_raw = deps.api.canonical_address(&spender)?;
    let owner_raw = &env.message.sender;

    allowances(&mut deps.storage, owner_raw).update(spender_raw.as_slice(), |allow| {
        let mut val = allow.unwrap_or_default();
        if let Some(exp) = expires {
            val.expires = exp;
        }
        val.allowance += amount;
        Ok(val)
    })?;

    let res = HandleResponse {
        messages: vec![],
        log: vec![
            log("action", "increase_allowance"),
            log("owner", deps.api.human_address(owner_raw)?),
            log("spender", spender),
            log("amount", amount),
        ],
        data: None,
    };
    Ok(res)
}

pub fn handle_decrease_allowance<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    spender: HumanAddr,
    amount: Uint128,
    expires: Option<Expiration>,
) -> StdResult<HandleResponse> {
    let spender_raw = deps.api.canonical_address(&spender)?;
    let owner_raw = &env.message.sender;

    // load value and delete if it hits 0, or update otherwise
    let mut bucket = allowances(&mut deps.storage, owner_raw);
    let mut allowance = bucket.load(spender_raw.as_slice())?;
    if amount < allowance.allowance {
        // update the new amount
        allowance.allowance = (allowance.allowance - amount)?;
        if let Some(exp) = expires {
            allowance.expires = exp;
        }
        bucket.save(spender_raw.as_slice(), &allowance)?;
    } else {
        allowance_remove(&mut deps.storage, owner_raw, &spender_raw);
    }

    let res = HandleResponse {
        messages: vec![],
        log: vec![
            log("action", "decrease_allowance"),
            log("owner", deps.api.human_address(owner_raw)?),
            log("spender", spender),
            log("amount", amount),
        ],
        data: None,
    };
    Ok(res)
}

// this can be used to update a lower allowance - call bucket.update with proper keys
fn deduct_allowance<S: Storage>(
    storage: &mut S,
    owner: &CanonicalAddr,
    spender: &CanonicalAddr,
    block: &BlockInfo,
    amount: Uint128,
) -> StdResult<AllowanceResponse> {
    allowances(storage, owner).update(spender.as_slice(), |current| {
        match current {
            Some(mut a) => {
                if a.expires.is_expired(block) {
                    Err(StdError::generic_err("Allowance is expired"))
                } else {
                    // deduct the allowance if enough
                    a.allowance = (a.allowance - amount)?;
                    Ok(a)
                }
            }
            None => Err(StdError::generic_err("No allowance for this account")),
        }
    })
}

pub fn handle_transfer_from<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    owner: HumanAddr,
    recipient: HumanAddr,
    amount: Uint128,
) -> StdResult<HandleResponse> {
    let rcpt_raw = deps.api.canonical_address(&recipient)?;
    let owner_raw = deps.api.canonical_address(&owner)?;
    let spender_raw = env.message.sender;

    // deduct allowance before doing anything else have enough allowance
    deduct_allowance(
        &mut deps.storage,
        &owner_raw,
        &spender_raw,
        &env.block,
        amount,
    )?;

    let mut accounts = balances(&mut deps.storage);
    accounts.update(owner_raw.as_slice(), |balance: Option<Uint128>| {
        balance.unwrap_or_default() - amount
    })?;
    accounts.update(rcpt_raw.as_slice(), |balance: Option<Uint128>| {
        Ok(balance.unwrap_or_default() + amount)
    })?;

    let res = HandleResponse {
        messages: vec![],
        log: vec![
            log("action", "transfer_from"),
            log("from", owner),
            log("to", recipient),
            log("by", deps.api.human_address(&spender_raw)?),
            log("amount", amount),
        ],
        data: None,
    };
    Ok(res)
}

pub fn handle_burn_from<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    owner: HumanAddr,
    amount: Uint128,
) -> StdResult<HandleResponse> {
    let spender_raw = env.message.sender;
    let owner_raw = deps.api.canonical_address(&owner)?;

    // deduct allowance before doing anything else have enough allowance
    deduct_allowance(
        &mut deps.storage,
        &owner_raw,
        &spender_raw,
        &env.block,
        amount,
    )?;

    // lower balance
    let mut accounts = balances(&mut deps.storage);
    accounts.update(owner_raw.as_slice(), |balance: Option<Uint128>| {
        balance.unwrap_or_default() - amount
    })?;
    // reduce total_supply
    meta(&mut deps.storage).update(|mut meta| {
        meta.total_supply = (meta.total_supply - amount)?;
        Ok(meta)
    })?;

    let res = HandleResponse {
        messages: vec![],
        log: vec![
            log("action", "burn_from"),
            log("from", owner),
            log("by", deps.api.human_address(&spender_raw)?),
            log("amount", amount),
        ],
        data: None,
    };
    Ok(res)
}

pub fn handle_send_from<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    owner: HumanAddr,
    contract: HumanAddr,
    amount: Uint128,
    msg: Option<Binary>,
) -> StdResult<HandleResponse> {
    let rcpt_raw = deps.api.canonical_address(&contract)?;
    let owner_raw = deps.api.canonical_address(&owner)?;
    let spender_raw = env.message.sender;

    // deduct allowance before doing anything else have enough allowance
    deduct_allowance(
        &mut deps.storage,
        &owner_raw,
        &spender_raw,
        &env.block,
        amount,
    )?;

    // move the tokens to the contract
    let mut accounts = balances(&mut deps.storage);
    accounts.update(owner_raw.as_slice(), |balance: Option<Uint128>| {
        balance.unwrap_or_default() - amount
    })?;
    accounts.update(rcpt_raw.as_slice(), |balance: Option<Uint128>| {
        Ok(balance.unwrap_or_default() + amount)
    })?;

    let logs = vec![
        log("action", "send_from"),
        log("from", &owner),
        log("to", &contract),
        log("by", deps.api.human_address(&spender_raw)?),
        log("amount", amount),
    ];

    // create a send message
    let msg = Cw20ReceiveMsg {
        sender: owner,
        amount,
        msg,
    }
    .into_cosmos_msg(contract)?;

    let res = HandleResponse {
        messages: vec![msg],
        log: logs,
        data: None,
    };
    Ok(res)
}

pub fn query_allowance<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    owner: HumanAddr,
    spender: HumanAddr,
) -> StdResult<AllowanceResponse> {
    let owner_raw = deps.api.canonical_address(&owner)?;
    let spender_raw = deps.api.canonical_address(&spender)?;
    let allowance = allowances_read(&deps.storage, &owner_raw)
        .may_load(spender_raw.as_slice())?
        .unwrap_or_default();
    Ok(allowance)
}
