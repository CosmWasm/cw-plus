#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, coins, to_binary, Addr, BankMsg, Binary, Deps, DepsMut, Env, MessageInfo, Response,
    StdError, StdResult, Uint128,
};

use cw2::set_contract_version;
use cw20_base::allowances::{
    deduct_allowance, execute_decrease_allowance, execute_increase_allowance, execute_send_from,
    execute_transfer_from, query_allowance,
};
use cw20_base::contract::{
    execute_burn, execute_mint, execute_send, execute_transfer, query_balance, query_token_info,
};
use cw20_base::state::{MinterData, TokenInfo, TOKEN_INFO};

use crate::curves::DecimalPlaces;
use crate::error::ContractError;
use crate::msg::{CurveFn, CurveInfoResponse, ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{CurveState, CURVE_STATE, CURVE_TYPE};
use cw_utils::{must_pay, nonpayable};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cw20-bonding";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // store token info using cw20-base format
    let data = TokenInfo {
        name: msg.name,
        symbol: msg.symbol,
        decimals: msg.decimals,
        total_supply: Uint128::zero(),
        // set self as minter, so we can properly execute mint and burn
        mint: Some(MinterData {
            minter: env.contract.address,
            cap: None,
        }),
    };
    TOKEN_INFO.save(deps.storage, &data)?;

    let places = DecimalPlaces::new(msg.decimals, msg.reserve_decimals);
    let supply = CurveState::new(msg.reserve_denom, places);
    CURVE_STATE.save(deps.storage, &supply)?;

    CURVE_TYPE.save(deps.storage, &msg.curve_type)?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    // default implementation stores curve info as enum, you can do something else in a derived
    // contract and just pass in your custom curve to do_execute
    let curve_type = CURVE_TYPE.load(deps.storage)?;
    let curve_fn = curve_type.to_curve_fn();
    do_execute(deps, env, info, msg, curve_fn)
}

/// We pull out logic here, so we can import this from another contract and set a different Curve.
/// This contacts sets a curve with an enum in InstantiateMsg and stored in state, but you may want
/// to use custom math not included - make this easily reusable
pub fn do_execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
    curve_fn: CurveFn,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Buy {} => execute_buy(deps, env, info, curve_fn),

        // we override these from cw20
        ExecuteMsg::Burn { amount } => Ok(execute_sell(deps, env, info, curve_fn, amount)?),
        ExecuteMsg::BurnFrom { owner, amount } => {
            Ok(execute_sell_from(deps, env, info, curve_fn, owner, amount)?)
        }

        // these all come from cw20-base to implement the cw20 standard
        ExecuteMsg::Transfer { recipient, amount } => {
            Ok(execute_transfer(deps, env, info, recipient, amount)?)
        }
        ExecuteMsg::Send {
            contract,
            amount,
            msg,
        } => Ok(execute_send(deps, env, info, contract, amount, msg)?),
        ExecuteMsg::IncreaseAllowance {
            spender,
            amount,
            expires,
        } => Ok(execute_increase_allowance(
            deps, env, info, spender, amount, expires,
        )?),
        ExecuteMsg::DecreaseAllowance {
            spender,
            amount,
            expires,
        } => Ok(execute_decrease_allowance(
            deps, env, info, spender, amount, expires,
        )?),
        ExecuteMsg::TransferFrom {
            owner,
            recipient,
            amount,
        } => Ok(execute_transfer_from(
            deps, env, info, owner, recipient, amount,
        )?),
        ExecuteMsg::SendFrom {
            owner,
            contract,
            amount,
            msg,
        } => Ok(execute_send_from(
            deps, env, info, owner, contract, amount, msg,
        )?),
    }
}

pub fn execute_buy(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    curve_fn: CurveFn,
) -> Result<Response, ContractError> {
    let mut state = CURVE_STATE.load(deps.storage)?;

    let payment = must_pay(&info, &state.reserve_denom)?;

    // calculate how many tokens can be purchased with this and mint them
    let curve = curve_fn(state.decimals);
    state.reserve += payment;
    let new_supply = curve.supply(state.reserve);
    let minted = new_supply
        .checked_sub(state.supply)
        .map_err(StdError::overflow)?;
    state.supply = new_supply;
    CURVE_STATE.save(deps.storage, &state)?;

    // call into cw20-base to mint the token, call as self as no one else is allowed
    let sub_info = MessageInfo {
        sender: env.contract.address.clone(),
        funds: vec![],
    };
    execute_mint(deps, env, sub_info, info.sender.to_string(), minted)?;

    // bond them to the validator
    let res = Response::new()
        .add_attribute("action", "buy")
        .add_attribute("from", info.sender)
        .add_attribute("reserve", payment)
        .add_attribute("supply", minted);
    Ok(res)
}

pub fn execute_sell(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    curve_fn: CurveFn,
    amount: Uint128,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    let receiver = info.sender.clone();
    // do all the work
    let mut res = do_sell(deps, env, info, curve_fn, receiver, amount)?;

    // add our custom attributes
    res.attributes.push(attr("action", "burn"));
    Ok(res)
}

pub fn execute_sell_from(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    curve_fn: CurveFn,
    owner: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    let owner_addr = deps.api.addr_validate(&owner)?;
    let spender_addr = info.sender.clone();

    // deduct allowance before doing anything else have enough allowance
    deduct_allowance(deps.storage, &owner_addr, &spender_addr, &env.block, amount)?;

    // do all the work in do_sell
    let receiver_addr = info.sender;
    let owner_info = MessageInfo {
        sender: owner_addr,
        funds: info.funds,
    };
    let mut res = do_sell(
        deps,
        env,
        owner_info,
        curve_fn,
        receiver_addr.clone(),
        amount,
    )?;

    // add our custom attributes
    res.attributes.push(attr("action", "burn_from"));
    res.attributes.push(attr("by", receiver_addr));
    Ok(res)
}

fn do_sell(
    mut deps: DepsMut,
    env: Env,
    // info.sender is the one burning tokens
    info: MessageInfo,
    curve_fn: CurveFn,
    // receiver is the one who gains (same for execute_sell, diff for execute_sell_from)
    receiver: Addr,
    amount: Uint128,
) -> Result<Response, ContractError> {
    // burn from the caller, this ensures there are tokens to cover this
    execute_burn(deps.branch(), env, info.clone(), amount)?;

    // calculate how many tokens can be purchased with this and mint them
    let mut state = CURVE_STATE.load(deps.storage)?;
    let curve = curve_fn(state.decimals);
    state.supply = state
        .supply
        .checked_sub(amount)
        .map_err(StdError::overflow)?;
    let new_reserve = curve.reserve(state.supply);
    let released = state
        .reserve
        .checked_sub(new_reserve)
        .map_err(StdError::overflow)?;
    state.reserve = new_reserve;
    CURVE_STATE.save(deps.storage, &state)?;

    // now send the tokens to the sender (TODO: for sell_from we do something else, right???)
    let msg = BankMsg::Send {
        to_address: receiver.to_string(),
        amount: coins(released.u128(), state.reserve_denom),
    };
    let res = Response::new()
        .add_message(msg)
        .add_attribute("from", info.sender)
        .add_attribute("supply", amount)
        .add_attribute("reserve", released);
    Ok(res)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    // default implementation stores curve info as enum, you can do something else in a derived
    // contract and just pass in your custom curve to do_execute
    let curve_type = CURVE_TYPE.load(deps.storage)?;
    let curve_fn = curve_type.to_curve_fn();
    do_query(deps, env, msg, curve_fn)
}

/// We pull out logic here, so we can import this from another contract and set a different Curve.
/// This contacts sets a curve with an enum in InstantitateMsg and stored in state, but you may want
/// to use custom math not included - make this easily reusable
pub fn do_query(deps: Deps, _env: Env, msg: QueryMsg, curve_fn: CurveFn) -> StdResult<Binary> {
    match msg {
        // custom queries
        QueryMsg::CurveInfo {} => to_binary(&query_curve_info(deps, curve_fn)?),
        // inherited from cw20-base
        QueryMsg::TokenInfo {} => to_binary(&query_token_info(deps)?),
        QueryMsg::Balance { address } => to_binary(&query_balance(deps, address)?),
        QueryMsg::Allowance { owner, spender } => {
            to_binary(&query_allowance(deps, owner, spender)?)
        }
    }
}

pub fn query_curve_info(deps: Deps, curve_fn: CurveFn) -> StdResult<CurveInfoResponse> {
    let CurveState {
        reserve,
        supply,
        reserve_denom,
        decimals,
    } = CURVE_STATE.load(deps.storage)?;

    // This we can get from the local digits stored in instantiate
    let curve = curve_fn(decimals);
    let spot_price = curve.spot_price(supply);

    Ok(CurveInfoResponse {
        reserve,
        supply,
        spot_price,
        reserve_denom,
    })
}

// this is poor mans "skip" flag
#[cfg(test)]
mod tests {
    use super::*;
    use crate::msg::CurveType;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coin, Decimal, OverflowError, OverflowOperation, StdError, SubMsg};
    use cw_utils::PaymentError;

    const DENOM: &str = "satoshi";
    const CREATOR: &str = "creator";
    const INVESTOR: &str = "investor";
    const BUYER: &str = "buyer";

    fn default_instantiate(
        decimals: u8,
        reserve_decimals: u8,
        curve_type: CurveType,
    ) -> InstantiateMsg {
        InstantiateMsg {
            name: "Bonded".to_string(),
            symbol: "EPOXY".to_string(),
            decimals,
            reserve_denom: DENOM.to_string(),
            reserve_decimals,
            curve_type,
        }
    }

    fn get_balance<U: Into<String>>(deps: Deps, addr: U) -> Uint128 {
        query_balance(deps, addr.into()).unwrap().balance
    }

    fn setup_test(deps: DepsMut, decimals: u8, reserve_decimals: u8, curve_type: CurveType) {
        // this matches `linear_curve` test case from curves.rs
        let creator = String::from(CREATOR);
        let msg = default_instantiate(decimals, reserve_decimals, curve_type);
        let info = mock_info(&creator, &[]);

        // make sure we can instantiate with this
        let res = instantiate(deps, mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
    }

    #[test]
    fn proper_instantiation() {
        let mut deps = mock_dependencies();

        // this matches `linear_curve` test case from curves.rs
        let creator = String::from("creator");
        let curve_type = CurveType::SquareRoot {
            slope: Uint128::new(1),
            scale: 1,
        };
        let msg = default_instantiate(2, 8, curve_type.clone());
        let info = mock_info(&creator, &[]);

        // make sure we can instantiate with this
        let res = instantiate(deps.as_mut(), mock_env(), info, msg.clone()).unwrap();
        assert_eq!(0, res.messages.len());

        // token info is proper
        let token = query_token_info(deps.as_ref()).unwrap();
        assert_eq!(&token.name, &msg.name);
        assert_eq!(&token.symbol, &msg.symbol);
        assert_eq!(token.decimals, 2);
        assert_eq!(token.total_supply, Uint128::zero());

        // curve state is sensible
        let state = query_curve_info(deps.as_ref(), curve_type.to_curve_fn()).unwrap();
        assert_eq!(state.reserve, Uint128::zero());
        assert_eq!(state.supply, Uint128::zero());
        assert_eq!(state.reserve_denom.as_str(), DENOM);
        // spot price 0 as supply is 0
        assert_eq!(state.spot_price, Decimal::zero());

        // curve type is stored properly
        let curve = CURVE_TYPE.load(&deps.storage).unwrap();
        assert_eq!(curve_type, curve);

        // no balance
        assert_eq!(get_balance(deps.as_ref(), &creator), Uint128::zero());
    }

    #[test]
    fn buy_issues_tokens() {
        let mut deps = mock_dependencies();
        let curve_type = CurveType::Linear {
            slope: Uint128::new(1),
            scale: 1,
        };
        setup_test(deps.as_mut(), 2, 8, curve_type.clone());

        // succeeds with proper token (5 BTC = 5*10^8 satoshi)
        let info = mock_info(INVESTOR, &coins(500_000_000, DENOM));
        let buy = ExecuteMsg::Buy {};
        execute(deps.as_mut(), mock_env(), info, buy.clone()).unwrap();

        // bob got 1000 EPOXY (10.00)
        assert_eq!(get_balance(deps.as_ref(), INVESTOR), Uint128::new(1000));
        assert_eq!(get_balance(deps.as_ref(), BUYER), Uint128::zero());

        // send them all to buyer
        let info = mock_info(INVESTOR, &[]);
        let send = ExecuteMsg::Transfer {
            recipient: BUYER.into(),
            amount: Uint128::new(1000),
        };
        execute(deps.as_mut(), mock_env(), info, send).unwrap();

        // ensure balances updated
        assert_eq!(get_balance(deps.as_ref(), INVESTOR), Uint128::zero());
        assert_eq!(get_balance(deps.as_ref(), BUYER), Uint128::new(1000));

        // second stake needs more to get next 1000 EPOXY
        let info = mock_info(INVESTOR, &coins(1_500_000_000, DENOM));
        execute(deps.as_mut(), mock_env(), info, buy).unwrap();

        // ensure balances updated
        assert_eq!(get_balance(deps.as_ref(), INVESTOR), Uint128::new(1000));
        assert_eq!(get_balance(deps.as_ref(), BUYER), Uint128::new(1000));

        // check curve info updated
        let curve = query_curve_info(deps.as_ref(), curve_type.to_curve_fn()).unwrap();
        assert_eq!(curve.reserve, Uint128::new(2_000_000_000));
        assert_eq!(curve.supply, Uint128::new(2000));
        assert_eq!(curve.spot_price, Decimal::percent(200));

        // check token info updated
        let token = query_token_info(deps.as_ref()).unwrap();
        assert_eq!(token.decimals, 2);
        assert_eq!(token.total_supply, Uint128::new(2000));
    }

    #[test]
    fn bonding_fails_with_wrong_denom() {
        let mut deps = mock_dependencies();
        let curve_type = CurveType::Linear {
            slope: Uint128::new(1),
            scale: 1,
        };
        setup_test(deps.as_mut(), 2, 8, curve_type);

        // fails when no tokens sent
        let info = mock_info(INVESTOR, &[]);
        let buy = ExecuteMsg::Buy {};
        let err = execute(deps.as_mut(), mock_env(), info, buy.clone()).unwrap_err();
        assert_eq!(err, PaymentError::NoFunds {}.into());

        // fails when wrong tokens sent
        let info = mock_info(INVESTOR, &coins(1234567, "wei"));
        let err = execute(deps.as_mut(), mock_env(), info, buy.clone()).unwrap_err();
        assert_eq!(err, PaymentError::MissingDenom(DENOM.into()).into());

        // fails when too many tokens sent
        let info = mock_info(INVESTOR, &[coin(3400022, DENOM), coin(1234567, "wei")]);
        let err = execute(deps.as_mut(), mock_env(), info, buy).unwrap_err();
        assert_eq!(err, PaymentError::MultipleDenoms {}.into());
    }

    #[test]
    fn burning_sends_reserve() {
        let mut deps = mock_dependencies();
        let curve_type = CurveType::Linear {
            slope: Uint128::new(1),
            scale: 1,
        };
        setup_test(deps.as_mut(), 2, 8, curve_type.clone());

        // succeeds with proper token (20 BTC = 20*10^8 satoshi)
        let info = mock_info(INVESTOR, &coins(2_000_000_000, DENOM));
        let buy = ExecuteMsg::Buy {};
        execute(deps.as_mut(), mock_env(), info, buy).unwrap();

        // bob got 2000 EPOXY (20.00)
        assert_eq!(get_balance(deps.as_ref(), INVESTOR), Uint128::new(2000));

        // cannot burn too much
        let info = mock_info(INVESTOR, &[]);
        let burn = ExecuteMsg::Burn {
            amount: Uint128::new(3000),
        };
        let err = execute(deps.as_mut(), mock_env(), info, burn).unwrap_err();
        assert_eq!(
            err,
            ContractError::Base(cw20_base::ContractError::Std(StdError::overflow(
                OverflowError::new(OverflowOperation::Sub, 2000, 3000)
            )))
        );

        // burn 1000 EPOXY to get back 15BTC (*10^8)
        let info = mock_info(INVESTOR, &[]);
        let burn = ExecuteMsg::Burn {
            amount: Uint128::new(1000),
        };
        let res = execute(deps.as_mut(), mock_env(), info, burn).unwrap();

        // balance is lower
        assert_eq!(get_balance(deps.as_ref(), INVESTOR), Uint128::new(1000));

        // ensure we got our money back
        assert_eq!(1, res.messages.len());
        assert_eq!(
            &res.messages[0],
            &SubMsg::new(BankMsg::Send {
                to_address: INVESTOR.into(),
                amount: coins(1_500_000_000, DENOM),
            })
        );

        // check curve info updated
        let curve = query_curve_info(deps.as_ref(), curve_type.to_curve_fn()).unwrap();
        assert_eq!(curve.reserve, Uint128::new(500_000_000));
        assert_eq!(curve.supply, Uint128::new(1000));
        assert_eq!(curve.spot_price, Decimal::percent(100));

        // check token info updated
        let token = query_token_info(deps.as_ref()).unwrap();
        assert_eq!(token.decimals, 2);
        assert_eq!(token.total_supply, Uint128::new(1000));
    }

    #[test]
    fn cw20_imports_work() {
        let mut deps = mock_dependencies();
        let curve_type = CurveType::Constant {
            value: Uint128::new(15),
            scale: 1,
        };
        setup_test(deps.as_mut(), 9, 6, curve_type);

        let alice: &str = "alice";
        let bob: &str = "bobby";
        let carl: &str = "carl";

        // spend 45_000 uatom for 30_000_000 EPOXY
        let info = mock_info(bob, &coins(45_000, DENOM));
        let buy = ExecuteMsg::Buy {};
        execute(deps.as_mut(), mock_env(), info, buy).unwrap();

        // check balances
        assert_eq!(get_balance(deps.as_ref(), bob), Uint128::new(30_000_000));
        assert_eq!(get_balance(deps.as_ref(), carl), Uint128::zero());

        // send coins to carl
        let bob_info = mock_info(bob, &[]);
        let transfer = ExecuteMsg::Transfer {
            recipient: carl.into(),
            amount: Uint128::new(2_000_000),
        };
        execute(deps.as_mut(), mock_env(), bob_info.clone(), transfer).unwrap();
        assert_eq!(get_balance(deps.as_ref(), bob), Uint128::new(28_000_000));
        assert_eq!(get_balance(deps.as_ref(), carl), Uint128::new(2_000_000));

        // allow alice
        let allow = ExecuteMsg::IncreaseAllowance {
            spender: alice.into(),
            amount: Uint128::new(35_000_000),
            expires: None,
        };
        execute(deps.as_mut(), mock_env(), bob_info, allow).unwrap();
        assert_eq!(get_balance(deps.as_ref(), bob), Uint128::new(28_000_000));
        assert_eq!(get_balance(deps.as_ref(), alice), Uint128::zero());
        assert_eq!(
            query_allowance(deps.as_ref(), bob.into(), alice.into())
                .unwrap()
                .allowance,
            Uint128::new(35_000_000)
        );

        // alice takes some for herself
        let self_pay = ExecuteMsg::TransferFrom {
            owner: bob.into(),
            recipient: alice.into(),
            amount: Uint128::new(25_000_000),
        };
        let alice_info = mock_info(alice, &[]);
        execute(deps.as_mut(), mock_env(), alice_info, self_pay).unwrap();
        assert_eq!(get_balance(deps.as_ref(), bob), Uint128::new(3_000_000));
        assert_eq!(get_balance(deps.as_ref(), alice), Uint128::new(25_000_000));
        assert_eq!(get_balance(deps.as_ref(), carl), Uint128::new(2_000_000));
        assert_eq!(
            query_allowance(deps.as_ref(), bob.into(), alice.into())
                .unwrap()
                .allowance,
            Uint128::new(10_000_000)
        );

        // test burn from works properly (burn tested in burning_sends_reserve)
        // cannot burn more than they have

        let info = mock_info(alice, &[]);
        let burn_from = ExecuteMsg::BurnFrom {
            owner: bob.into(),
            amount: Uint128::new(3_300_000),
        };
        let err = execute(deps.as_mut(), mock_env(), info, burn_from).unwrap_err();
        assert_eq!(
            err,
            ContractError::Base(cw20_base::ContractError::Std(StdError::overflow(
                OverflowError::new(OverflowOperation::Sub, 3000000, 3300000)
            )))
        );

        // burn 1_000_000 EPOXY to get back 1_500 DENOM (constant curve)
        let info = mock_info(alice, &[]);
        let burn_from = ExecuteMsg::BurnFrom {
            owner: bob.into(),
            amount: Uint128::new(1_000_000),
        };
        let res = execute(deps.as_mut(), mock_env(), info, burn_from).unwrap();

        // bob balance is lower, not alice
        assert_eq!(get_balance(deps.as_ref(), alice), Uint128::new(25_000_000));
        assert_eq!(get_balance(deps.as_ref(), bob), Uint128::new(2_000_000));

        // ensure alice got our money back
        assert_eq!(1, res.messages.len());
        assert_eq!(
            &res.messages[0],
            &SubMsg::new(BankMsg::Send {
                to_address: alice.into(),
                amount: coins(1_500, DENOM),
            })
        );
    }
}
