use cosmwasm_std::{
    attr, Binary, BlockInfo, CanonicalAddr, Deps, DepsMut, Env, HumanAddr, MessageInfo, Response,
    StdResult, Storage, Uint128,
};
use cw20::{AllowanceResponse, Cw20ReceiveMsg, Expiration};

use crate::error::ContractError;
use crate::state::{ALLOWANCES, BALANCES, TOKEN_INFO};

pub fn execute_increase_allowance(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    spender: HumanAddr,
    amount: Uint128,
    expires: Option<Expiration>,
) -> Result<Response, ContractError> {
    let spender_raw = &deps.api.canonical_address(&spender)?;
    let owner_raw = &deps.api.canonical_address(&info.sender)?;

    if spender_raw == owner_raw {
        return Err(ContractError::CannotSetOwnAccount {});
    }

    ALLOWANCES.update(
        deps.storage,
        (&owner_raw, &spender_raw),
        |allow| -> StdResult<_> {
            let mut val = allow.unwrap_or_default();
            if let Some(exp) = expires {
                val.expires = exp;
            }
            val.allowance += amount;
            Ok(val)
        },
    )?;

    let res = Response {
        submessages: vec![],
        messages: vec![],
        attributes: vec![
            attr("action", "increase_allowance"),
            attr("owner", deps.api.human_address(owner_raw)?),
            attr("spender", spender),
            attr("amount", amount),
        ],
        data: None,
    };
    Ok(res)
}

pub fn execute_decrease_allowance(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    spender: HumanAddr,
    amount: Uint128,
    expires: Option<Expiration>,
) -> Result<Response, ContractError> {
    if spender == info.sender {
        return Err(ContractError::CannotSetOwnAccount {});
    }

    let spender_raw = &deps.api.canonical_address(&spender)?;
    let owner_raw = &deps.api.canonical_address(&info.sender)?;

    // load value and delete if it hits 0, or update otherwise
    let mut allowance = ALLOWANCES.load(deps.storage, (&owner_raw, &spender_raw))?;
    if amount < allowance.allowance {
        // update the new amount
        allowance.allowance = (allowance.allowance - amount)?;
        if let Some(exp) = expires {
            allowance.expires = exp;
        }
        ALLOWANCES.save(deps.storage, (&owner_raw, &spender_raw), &allowance)?;
    } else {
        ALLOWANCES.remove(deps.storage, (&owner_raw, &spender_raw));
    }

    let res = Response {
        submessages: vec![],
        messages: vec![],
        attributes: vec![
            attr("action", "decrease_allowance"),
            attr("owner", info.sender),
            attr("spender", spender),
            attr("amount", amount),
        ],
        data: None,
    };
    Ok(res)
}

// this can be used to update a lower allowance - call bucket.update with proper keys
pub fn deduct_allowance(
    storage: &mut dyn Storage,
    owner: &CanonicalAddr,
    spender: &CanonicalAddr,
    block: &BlockInfo,
    amount: Uint128,
) -> Result<AllowanceResponse, ContractError> {
    ALLOWANCES.update(storage, (&owner, &spender), |current| {
        match current {
            Some(mut a) => {
                if a.expires.is_expired(block) {
                    Err(ContractError::Expired {})
                } else {
                    // deduct the allowance if enough
                    a.allowance = (a.allowance - amount)?;
                    Ok(a)
                }
            }
            None => Err(ContractError::NoAllowance {}),
        }
    })
}

pub fn execute_transfer_from(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    owner: HumanAddr,
    recipient: HumanAddr,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let rcpt_raw = deps.api.canonical_address(&recipient)?;
    let owner_raw = deps.api.canonical_address(&owner)?;
    let spender_raw = deps.api.canonical_address(&info.sender)?;

    // deduct allowance before doing anything else have enough allowance
    deduct_allowance(deps.storage, &owner_raw, &spender_raw, &env.block, amount)?;

    BALANCES.update(deps.storage, &owner_raw, |balance: Option<Uint128>| {
        balance.unwrap_or_default() - amount
    })?;
    BALANCES.update(
        deps.storage,
        &rcpt_raw,
        |balance: Option<Uint128>| -> StdResult<_> { Ok(balance.unwrap_or_default() + amount) },
    )?;

    let res = Response {
        submessages: vec![],
        messages: vec![],
        attributes: vec![
            attr("action", "transfer_from"),
            attr("from", owner),
            attr("to", recipient),
            attr("by", deps.api.human_address(&spender_raw)?),
            attr("amount", amount),
        ],
        data: None,
    };
    Ok(res)
}

pub fn execute_burn_from(
    deps: DepsMut,

    env: Env,
    info: MessageInfo,
    owner: HumanAddr,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let owner_raw = deps.api.canonical_address(&owner)?;
    let spender_raw = deps.api.canonical_address(&info.sender)?;

    // deduct allowance before doing anything else have enough allowance
    deduct_allowance(deps.storage, &owner_raw, &spender_raw, &env.block, amount)?;

    // lower balance
    BALANCES.update(deps.storage, &owner_raw, |balance: Option<Uint128>| {
        balance.unwrap_or_default() - amount
    })?;
    // reduce total_supply
    TOKEN_INFO.update(deps.storage, |mut meta| -> StdResult<_> {
        meta.total_supply = (meta.total_supply - amount)?;
        Ok(meta)
    })?;

    let res = Response {
        submessages: vec![],
        messages: vec![],
        attributes: vec![
            attr("action", "burn_from"),
            attr("from", owner),
            attr("by", deps.api.human_address(&spender_raw)?),
            attr("amount", amount),
        ],
        data: None,
    };
    Ok(res)
}

pub fn execute_send_from(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    owner: HumanAddr,
    contract: HumanAddr,
    amount: Uint128,
    msg: Option<Binary>,
) -> Result<Response, ContractError> {
    let rcpt_raw = deps.api.canonical_address(&contract)?;
    let owner_raw = deps.api.canonical_address(&owner)?;
    let spender_raw = deps.api.canonical_address(&info.sender)?;

    // deduct allowance before doing anything else have enough allowance
    deduct_allowance(deps.storage, &owner_raw, &spender_raw, &env.block, amount)?;

    // move the tokens to the contract
    BALANCES.update(deps.storage, &owner_raw, |balance: Option<Uint128>| {
        balance.unwrap_or_default() - amount
    })?;
    BALANCES.update(
        deps.storage,
        &rcpt_raw,
        |balance: Option<Uint128>| -> StdResult<_> { Ok(balance.unwrap_or_default() + amount) },
    )?;

    let attrs = vec![
        attr("action", "send_from"),
        attr("from", &owner),
        attr("to", &contract),
        attr("by", &info.sender),
        attr("amount", amount),
    ];

    // create a send message
    let msg = Cw20ReceiveMsg {
        sender: info.sender,
        amount,
        msg,
    }
    .into_cosmos_msg(contract)?;

    let res = Response {
        submessages: vec![],
        messages: vec![msg],
        attributes: attrs,
        data: None,
    };
    Ok(res)
}

pub fn query_allowance(
    deps: Deps,
    owner: HumanAddr,
    spender: HumanAddr,
) -> StdResult<AllowanceResponse> {
    let owner_raw = deps.api.canonical_address(&owner)?;
    let spender_raw = deps.api.canonical_address(&spender)?;
    let allowance = ALLOWANCES
        .may_load(deps.storage, (&owner_raw, &spender_raw))?
        .unwrap_or_default();
    Ok(allowance)
}

#[cfg(test)]
mod tests {
    use super::*;

    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coins, CosmosMsg, StdError, WasmMsg};
    use cw20::{Cw20CoinHuman, TokenInfoResponse};

    use crate::contract::{execute, instantiate, query_balance, query_token_info};
    use crate::msg::{ExecuteMsg, InstantiateMsg};

    fn get_balance<T: Into<HumanAddr>>(deps: Deps, address: T) -> Uint128 {
        query_balance(deps, address.into()).unwrap().balance
    }

    // this will set up the instantiation for other tests
    fn do_instantiate(mut deps: DepsMut, addr: &HumanAddr, amount: Uint128) -> TokenInfoResponse {
        let instantiate_msg = InstantiateMsg {
            name: "Auto Gen".to_string(),
            symbol: "AUTO".to_string(),
            decimals: 3,
            initial_balances: vec![Cw20CoinHuman {
                address: addr.into(),
                amount,
            }],
            mint: None,
        };
        let info = mock_info(&HumanAddr("creator".to_string()), &[]);
        let env = mock_env();
        instantiate(deps.branch(), env, info, instantiate_msg).unwrap();
        query_token_info(deps.as_ref()).unwrap()
    }

    #[test]
    fn increase_decrease_allowances() {
        let mut deps = mock_dependencies(&coins(2, "token"));

        let owner = HumanAddr::from("addr0001");
        let spender = HumanAddr::from("addr0002");
        let info = mock_info(owner.clone(), &[]);
        let env = mock_env();
        do_instantiate(deps.as_mut(), &owner, Uint128(12340000));

        // no allowance to start
        let allowance = query_allowance(deps.as_ref(), owner.clone(), spender.clone()).unwrap();
        assert_eq!(allowance, AllowanceResponse::default());

        // set allowance with height expiration
        let allow1 = Uint128(7777);
        let expires = Expiration::AtHeight(5432);
        let msg = ExecuteMsg::IncreaseAllowance {
            spender: spender.clone(),
            amount: allow1,
            expires: Some(expires.clone()),
        };
        execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        // ensure it looks good
        let allowance = query_allowance(deps.as_ref(), owner.clone(), spender.clone()).unwrap();
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
        let msg = ExecuteMsg::DecreaseAllowance {
            spender: spender.clone(),
            amount: lower,
            expires: None,
        };
        execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
        let allowance = query_allowance(deps.as_ref(), owner.clone(), spender.clone()).unwrap();
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
        let new_expire = Expiration::AtTime(8888888888);
        let msg = ExecuteMsg::IncreaseAllowance {
            spender: spender.clone(),
            amount: raise,
            expires: Some(new_expire.clone()),
        };
        execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
        let allowance = query_allowance(deps.as_ref(), owner.clone(), spender.clone()).unwrap();
        assert_eq!(
            allowance,
            AllowanceResponse {
                allowance: allow3,
                expires: new_expire.clone()
            }
        );

        // decrease it below 0
        let msg = ExecuteMsg::DecreaseAllowance {
            spender: spender.clone(),
            amount: Uint128(99988647623876347),
            expires: None,
        };
        execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
        let allowance = query_allowance(deps.as_ref(), owner.clone(), spender.clone()).unwrap();
        assert_eq!(allowance, AllowanceResponse::default());
    }

    #[test]
    fn allowances_independent() {
        let mut deps = mock_dependencies(&coins(2, "token"));

        let owner = HumanAddr::from("addr0001");
        let spender = HumanAddr::from("addr0002");
        let spender2 = HumanAddr::from("addr0003");
        let info = mock_info(owner.clone(), &[]);
        let env = mock_env();
        do_instantiate(deps.as_mut(), &owner, Uint128(12340000));

        // no allowance to start
        assert_eq!(
            query_allowance(deps.as_ref(), owner.clone(), spender.clone()).unwrap(),
            AllowanceResponse::default()
        );
        assert_eq!(
            query_allowance(deps.as_ref(), owner.clone(), spender2.clone()).unwrap(),
            AllowanceResponse::default()
        );
        assert_eq!(
            query_allowance(deps.as_ref(), spender.clone(), spender2.clone()).unwrap(),
            AllowanceResponse::default()
        );

        // set allowance with height expiration
        let allow1 = Uint128(7777);
        let expires = Expiration::AtHeight(5432);
        let msg = ExecuteMsg::IncreaseAllowance {
            spender: spender.clone(),
            amount: allow1,
            expires: Some(expires.clone()),
        };
        execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        // set other allowance with no expiration
        let allow2 = Uint128(87654);
        let msg = ExecuteMsg::IncreaseAllowance {
            spender: spender2.clone(),
            amount: allow2,
            expires: None,
        };
        execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

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
            query_allowance(deps.as_ref(), owner.clone(), spender.clone()).unwrap(),
            expect_one.clone()
        );
        assert_eq!(
            query_allowance(deps.as_ref(), owner.clone(), spender2.clone()).unwrap(),
            expect_two.clone()
        );
        assert_eq!(
            query_allowance(deps.as_ref(), spender.clone(), spender2.clone()).unwrap(),
            AllowanceResponse::default()
        );

        // also allow spender -> spender2 with no interference
        let info = mock_info(spender.clone(), &[]);
        let env = mock_env();
        let allow3 = Uint128(1821);
        let expires3 = Expiration::AtTime(3767626296);
        let msg = ExecuteMsg::IncreaseAllowance {
            spender: spender2.clone(),
            amount: allow3,
            expires: Some(expires3.clone()),
        };
        execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
        let expect_three = AllowanceResponse {
            allowance: allow3,
            expires: expires3,
        };
        assert_eq!(
            query_allowance(deps.as_ref(), owner.clone(), spender.clone()).unwrap(),
            expect_one.clone()
        );
        assert_eq!(
            query_allowance(deps.as_ref(), owner.clone(), spender2.clone()).unwrap(),
            expect_two.clone()
        );
        assert_eq!(
            query_allowance(deps.as_ref(), spender.clone(), spender2.clone()).unwrap(),
            expect_three.clone()
        );
    }

    #[test]
    fn no_self_allowance() {
        let mut deps = mock_dependencies(&coins(2, "token"));

        let owner = HumanAddr::from("addr0001");
        let info = mock_info(owner.clone(), &[]);
        let env = mock_env();
        do_instantiate(deps.as_mut(), &owner, Uint128(12340000));

        // self-allowance
        let msg = ExecuteMsg::IncreaseAllowance {
            spender: owner.clone(),
            amount: Uint128(7777),
            expires: None,
        };
        let err = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap_err();
        assert_eq!(err, ContractError::CannotSetOwnAccount {});

        // decrease self-allowance
        let msg = ExecuteMsg::DecreaseAllowance {
            spender: owner.clone(),
            amount: Uint128(7777),
            expires: None,
        };
        let err = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap_err();
        assert_eq!(err, ContractError::CannotSetOwnAccount {});
    }

    #[test]
    fn transfer_from_respects_limits() {
        let mut deps = mock_dependencies(&[]);
        let owner = HumanAddr::from("addr0001");
        let spender = HumanAddr::from("addr0002");
        let rcpt = HumanAddr::from("addr0003");

        let start = Uint128(999999);
        do_instantiate(deps.as_mut(), &owner, start);

        // provide an allowance
        let allow1 = Uint128(77777);
        let msg = ExecuteMsg::IncreaseAllowance {
            spender: spender.clone(),
            amount: allow1,
            expires: None,
        };
        let info = mock_info(owner.clone(), &[]);
        let env = mock_env();
        execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        // valid transfer of part of the allowance
        let transfer = Uint128(44444);
        let msg = ExecuteMsg::TransferFrom {
            owner: owner.clone(),
            recipient: rcpt.clone(),
            amount: transfer,
        };
        let info = mock_info(spender.clone(), &[]);
        let env = mock_env();
        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
        assert_eq!(res.attributes[0], attr("action", "transfer_from"));

        // make sure money arrived
        assert_eq!(
            get_balance(deps.as_ref(), &owner),
            (start - transfer).unwrap()
        );
        assert_eq!(get_balance(deps.as_ref(), &rcpt), transfer);

        // ensure it looks good
        let allowance = query_allowance(deps.as_ref(), owner.clone(), spender.clone()).unwrap();
        let expect = AllowanceResponse {
            allowance: (allow1 - transfer).unwrap(),
            expires: Expiration::Never {},
        };
        assert_eq!(expect, allowance);

        // cannot send more than the allowance
        let msg = ExecuteMsg::TransferFrom {
            owner: owner.clone(),
            recipient: rcpt.clone(),
            amount: Uint128(33443),
        };
        let info = mock_info(spender.clone(), &[]);
        let env = mock_env();
        let err = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap_err();
        assert!(matches!(
            err,
            ContractError::Std(StdError::Underflow { .. })
        ));

        // let us increase limit, but set the expiration (default env height is 12_345)
        let info = mock_info(owner.clone(), &[]);
        let env = mock_env();
        let msg = ExecuteMsg::IncreaseAllowance {
            spender: spender.clone(),
            amount: Uint128(1000),
            expires: Some(Expiration::AtHeight(env.block.height)),
        };
        execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        // we should now get the expiration error
        let msg = ExecuteMsg::TransferFrom {
            owner: owner.clone(),
            recipient: rcpt.clone(),
            amount: Uint128(33443),
        };
        let info = mock_info(spender.clone(), &[]);
        let env = mock_env();
        let err = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap_err();
        assert_eq!(err, ContractError::Expired {});
    }

    #[test]
    fn burn_from_respects_limits() {
        let mut deps = mock_dependencies(&[]);
        let owner = HumanAddr::from("addr0001");
        let spender = HumanAddr::from("addr0002");

        let start = Uint128(999999);
        do_instantiate(deps.as_mut(), &owner, start);

        // provide an allowance
        let allow1 = Uint128(77777);
        let msg = ExecuteMsg::IncreaseAllowance {
            spender: spender.clone(),
            amount: allow1,
            expires: None,
        };
        let info = mock_info(owner.clone(), &[]);
        let env = mock_env();
        execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        // valid burn of part of the allowance
        let transfer = Uint128(44444);
        let msg = ExecuteMsg::BurnFrom {
            owner: owner.clone(),
            amount: transfer,
        };
        let info = mock_info(spender.clone(), &[]);
        let env = mock_env();
        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
        assert_eq!(res.attributes[0], attr("action", "burn_from"));

        // make sure money burnt
        assert_eq!(
            get_balance(deps.as_ref(), &owner),
            (start - transfer).unwrap()
        );

        // ensure it looks good
        let allowance = query_allowance(deps.as_ref(), owner.clone(), spender.clone()).unwrap();
        let expect = AllowanceResponse {
            allowance: (allow1 - transfer).unwrap(),
            expires: Expiration::Never {},
        };
        assert_eq!(expect, allowance);

        // cannot burn more than the allowance
        let msg = ExecuteMsg::BurnFrom {
            owner: owner.clone(),
            amount: Uint128(33443),
        };
        let info = mock_info(spender.clone(), &[]);
        let env = mock_env();
        let err = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap_err();
        assert!(matches!(
            err,
            ContractError::Std(StdError::Underflow { .. })
        ));

        // let us increase limit, but set the expiration (default env height is 12_345)
        let info = mock_info(owner.clone(), &[]);
        let env = mock_env();
        let msg = ExecuteMsg::IncreaseAllowance {
            spender: spender.clone(),
            amount: Uint128(1000),
            expires: Some(Expiration::AtHeight(env.block.height)),
        };
        execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        // we should now get the expiration error
        let msg = ExecuteMsg::BurnFrom {
            owner: owner.clone(),
            amount: Uint128(33443),
        };
        let info = mock_info(spender.clone(), &[]);
        let env = mock_env();
        let err = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap_err();
        assert_eq!(err, ContractError::Expired {});
    }

    #[test]
    fn send_from_respects_limits() {
        let mut deps = mock_dependencies(&[]);
        let owner = HumanAddr::from("addr0001");
        let spender = HumanAddr::from("addr0002");
        let contract = HumanAddr::from("cool-dex");
        let send_msg = Binary::from(r#"{"some":123}"#.as_bytes());

        let start = Uint128(999999);
        do_instantiate(deps.as_mut(), &owner, start);

        // provide an allowance
        let allow1 = Uint128(77777);
        let msg = ExecuteMsg::IncreaseAllowance {
            spender: spender.clone(),
            amount: allow1,
            expires: None,
        };
        let info = mock_info(owner.clone(), &[]);
        let env = mock_env();
        execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        // valid send of part of the allowance
        let transfer = Uint128(44444);
        let msg = ExecuteMsg::SendFrom {
            owner: owner.clone(),
            amount: transfer,
            contract: contract.clone(),
            msg: Some(send_msg.clone()),
        };
        let info = mock_info(spender.clone(), &[]);
        let env = mock_env();
        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
        assert_eq!(res.attributes[0], attr("action", "send_from"));
        assert_eq!(1, res.messages.len());

        // we record this as sent by the one who requested, not the one who was paying
        let binary_msg = Cw20ReceiveMsg {
            sender: spender.clone(),
            amount: transfer,
            msg: Some(send_msg.clone()),
        }
        .into_binary()
        .unwrap();
        assert_eq!(
            res.messages[0],
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: contract.clone(),
                msg: binary_msg,
                send: vec![],
            })
        );

        // make sure money sent
        assert_eq!(
            get_balance(deps.as_ref(), &owner),
            (start - transfer).unwrap()
        );
        assert_eq!(get_balance(deps.as_ref(), &contract), transfer);

        // ensure it looks good
        let allowance = query_allowance(deps.as_ref(), owner.clone(), spender.clone()).unwrap();
        let expect = AllowanceResponse {
            allowance: (allow1 - transfer).unwrap(),
            expires: Expiration::Never {},
        };
        assert_eq!(expect, allowance);

        // cannot send more than the allowance
        let msg = ExecuteMsg::SendFrom {
            owner: owner.clone(),
            amount: Uint128(33443),
            contract: contract.clone(),
            msg: Some(send_msg.clone()),
        };
        let info = mock_info(spender.clone(), &[]);
        let env = mock_env();
        let err = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap_err();
        assert!(matches!(
            err,
            ContractError::Std(StdError::Underflow { .. })
        ));

        // let us increase limit, but set the expiration to current block (expired)
        let info = mock_info(owner.clone(), &[]);
        let env = mock_env();
        let msg = ExecuteMsg::IncreaseAllowance {
            spender: spender.clone(),
            amount: Uint128(1000),
            expires: Some(Expiration::AtHeight(env.block.height)),
        };
        execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        // we should now get the expiration error
        let msg = ExecuteMsg::SendFrom {
            owner: owner.clone(),
            amount: Uint128(33443),
            contract: contract.clone(),
            msg: Some(send_msg.clone()),
        };
        let info = mock_info(spender.clone(), &[]);
        let env = mock_env();
        let err = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap_err();
        assert_eq!(err, ContractError::Expired {});
    }
}
