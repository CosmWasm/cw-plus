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

#[cfg(test)]
mod tests {
    use super::*;

    use cosmwasm_std::testing::{mock_dependencies, mock_env};
    use cosmwasm_std::{coins, StdError};
    use cw20::MetaResponse;

    use crate::contract::{handle, init, query_balance, query_meta};
    use crate::msg::{HandleMsg, InitMsg, InitialBalance};

    fn get_balance<S: Storage, A: Api, Q: Querier, T: Into<HumanAddr>>(
        deps: &Extern<S, A, Q>,
        address: T,
    ) -> Uint128 {
        query_balance(&deps, address.into()).unwrap().balance
    }

    // this will set up the init for other tests
    fn do_init<S: Storage, A: Api, Q: Querier>(
        deps: &mut Extern<S, A, Q>,
        addr: &HumanAddr,
        amount: Uint128,
    ) -> MetaResponse {
        let init_msg = InitMsg {
            name: "Auto Gen".to_string(),
            symbol: "AUTO".to_string(),
            decimals: 3,
            initial_balances: vec![InitialBalance {
                address: addr.into(),
                amount,
            }],
            mint: None,
        };
        let env = mock_env(&deps.api, &HumanAddr("creator".to_string()), &[]);
        init(deps, env, init_msg).unwrap();
        query_meta(&deps).unwrap()
    }

    #[test]
    fn increase_decrease_allowances() {
        let mut deps = mock_dependencies(20, &coins(2, "token"));

        let owner = HumanAddr::from("addr0001");
        let spender = HumanAddr::from("addr0002");
        let env = mock_env(&deps.api, owner.clone(), &[]);
        do_init(&mut deps, &owner, Uint128(12340000));

        // no allowance to start
        let allowance = query_allowance(&deps, owner.clone(), spender.clone()).unwrap();
        assert_eq!(allowance, AllowanceResponse::default());

        // set allowance with height expiration
        let allow1 = Uint128(7777);
        let expires = Expiration::AtHeight { height: 5432 };
        let msg = HandleMsg::IncreaseAllowance {
            spender: spender.clone(),
            amount: allow1,
            expires: Some(expires.clone()),
        };
        handle(&mut deps, env.clone(), msg).unwrap();

        // ensure it looks good
        let allowance = query_allowance(&deps, owner.clone(), spender.clone()).unwrap();
        assert_eq!(
            allowance,
            AllowanceResponse {
                allowance: allow1,
                expires: expires.clone()
            }
        );

        // decrease it a bit with no expire set - stays the same
        let lower = Uint128(4444);
        let allow2 = (allow1 - lower).unwrap();
        let msg = HandleMsg::DecreaseAllowance {
            spender: spender.clone(),
            amount: lower,
            expires: None,
        };
        handle(&mut deps, env.clone(), msg).unwrap();
        let allowance = query_allowance(&deps, owner.clone(), spender.clone()).unwrap();
        assert_eq!(
            allowance,
            AllowanceResponse {
                allowance: allow2,
                expires: expires.clone()
            }
        );

        // increase it some more and override the expires
        let raise = Uint128(87654);
        let allow3 = allow2 + raise;
        let new_expire = Expiration::AtTime { time: 8888888888 };
        let msg = HandleMsg::IncreaseAllowance {
            spender: spender.clone(),
            amount: raise,
            expires: Some(new_expire.clone()),
        };
        handle(&mut deps, env.clone(), msg).unwrap();
        let allowance = query_allowance(&deps, owner.clone(), spender.clone()).unwrap();
        assert_eq!(
            allowance,
            AllowanceResponse {
                allowance: allow3,
                expires: new_expire.clone()
            }
        );

        // decrease it below 0
        let msg = HandleMsg::DecreaseAllowance {
            spender: spender.clone(),
            amount: Uint128(99988647623876347),
            expires: None,
        };
        handle(&mut deps, env.clone(), msg).unwrap();
        let allowance = query_allowance(&deps, owner.clone(), spender.clone()).unwrap();
        assert_eq!(allowance, AllowanceResponse::default());
    }

    #[test]
    fn allowances_independent() {
        let mut deps = mock_dependencies(20, &coins(2, "token"));

        let owner = HumanAddr::from("addr0001");
        let spender = HumanAddr::from("addr0002");
        let spender2 = HumanAddr::from("addr0003");
        let env = mock_env(&deps.api, owner.clone(), &[]);
        do_init(&mut deps, &owner, Uint128(12340000));

        // no allowance to start
        assert_eq!(
            query_allowance(&deps, owner.clone(), spender.clone()).unwrap(),
            AllowanceResponse::default()
        );
        assert_eq!(
            query_allowance(&deps, owner.clone(), spender2.clone()).unwrap(),
            AllowanceResponse::default()
        );
        assert_eq!(
            query_allowance(&deps, spender.clone(), spender2.clone()).unwrap(),
            AllowanceResponse::default()
        );

        // set allowance with height expiration
        let allow1 = Uint128(7777);
        let expires = Expiration::AtHeight { height: 5432 };
        let msg = HandleMsg::IncreaseAllowance {
            spender: spender.clone(),
            amount: allow1,
            expires: Some(expires.clone()),
        };
        handle(&mut deps, env.clone(), msg).unwrap();

        // set other allowance with no expiration
        let allow2 = Uint128(87654);
        let msg = HandleMsg::IncreaseAllowance {
            spender: spender2.clone(),
            amount: allow2,
            expires: None,
        };
        handle(&mut deps, env.clone(), msg).unwrap();

        // check they are proper
        let expect_one = AllowanceResponse {
            allowance: allow1,
            expires,
        };
        let expect_two = AllowanceResponse {
            allowance: allow2,
            expires: Expiration::Never {},
        };
        assert_eq!(
            query_allowance(&deps, owner.clone(), spender.clone()).unwrap(),
            expect_one.clone()
        );
        assert_eq!(
            query_allowance(&deps, owner.clone(), spender2.clone()).unwrap(),
            expect_two.clone()
        );
        assert_eq!(
            query_allowance(&deps, spender.clone(), spender2.clone()).unwrap(),
            AllowanceResponse::default()
        );

        // also allow spender -> spender2 with no interference
        let env = mock_env(&deps.api, spender.clone(), &[]);
        let allow3 = Uint128(1821);
        let expires3 = Expiration::AtTime { time: 3767626296 };
        let msg = HandleMsg::IncreaseAllowance {
            spender: spender2.clone(),
            amount: allow3,
            expires: Some(expires3.clone()),
        };
        handle(&mut deps, env.clone(), msg).unwrap();
        let expect_three = AllowanceResponse {
            allowance: allow3,
            expires: expires3,
        };
        assert_eq!(
            query_allowance(&deps, owner.clone(), spender.clone()).unwrap(),
            expect_one.clone()
        );
        assert_eq!(
            query_allowance(&deps, owner.clone(), spender2.clone()).unwrap(),
            expect_two.clone()
        );
        assert_eq!(
            query_allowance(&deps, spender.clone(), spender2.clone()).unwrap(),
            expect_three.clone()
        );
    }

    #[test]
    fn no_self_allowance() {}

    #[test]
    fn transfer() {
        let mut deps = mock_dependencies(20, &coins(2, "token"));
        let addr1 = HumanAddr::from("addr0001");
        let addr2 = HumanAddr::from("addr0002");
        let amount1 = Uint128::from(12340000u128);
        let transfer = Uint128::from(76543u128);
        let too_much = Uint128::from(12340321u128);

        do_init(&mut deps, &addr1, amount1);

        // cannot send more than we have
        let env = mock_env(&deps.api, addr1.clone(), &[]);
        let msg = HandleMsg::Transfer {
            recipient: addr2.clone(),
            amount: too_much,
        };
        let res = handle(&mut deps, env, msg);
        match res.unwrap_err() {
            StdError::Underflow { .. } => {}
            e => panic!("Unexpected error: {}", e),
        }

        // cannot send from empty account
        let env = mock_env(&deps.api, addr2.clone(), &[]);
        let msg = HandleMsg::Transfer {
            recipient: addr1.clone(),
            amount: transfer,
        };
        let res = handle(&mut deps, env, msg);
        match res.unwrap_err() {
            StdError::Underflow { .. } => {}
            e => panic!("Unexpected error: {}", e),
        }

        // valid transfer
        let env = mock_env(&deps.api, addr1.clone(), &[]);
        let msg = HandleMsg::Transfer {
            recipient: addr2.clone(),
            amount: transfer,
        };
        let res = handle(&mut deps, env, msg).unwrap();
        assert_eq!(res.messages.len(), 0);

        let remainder = (amount1 - transfer).unwrap();
        assert_eq!(get_balance(&deps, &addr1), remainder);
        assert_eq!(get_balance(&deps, &addr2), transfer);
        assert_eq!(query_meta(&deps).unwrap().total_supply, amount1);
    }
}
