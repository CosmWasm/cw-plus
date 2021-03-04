use cosmwasm_std::{
    attr, coins, to_binary, BankMsg, Binary, Deps, DepsMut, Env, HumanAddr, MessageInfo, Response,
    StdResult, Uint128,
};

use cw2::set_contract_version;
use cw20_base::allowances::{
    deduct_allowance, handle_decrease_allowance, handle_increase_allowance, handle_send_from,
    handle_transfer_from, query_allowance,
};
use cw20_base::contract::{
    handle_burn, handle_mint, handle_send, handle_transfer, query_balance, query_token_info,
};
use cw20_base::state::{token_info, MinterData, TokenInfo};

use crate::curves::DecimalPlaces;
use crate::error::ContractError;
use crate::msg::{CurveFn, CurveInfoResponse, HandleMsg, InitMsg, QueryMsg};
use crate::state::{CurveState, CURVE_STATE, CURVE_TYPE};
use cw0::{must_pay, nonpayable};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cw20-bonding";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn init(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InitMsg,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // store token info using cw20-base format
    let data = TokenInfo {
        name: msg.name,
        symbol: msg.symbol,
        decimals: msg.decimals,
        total_supply: Uint128(0),
        // set self as minter, so we can properly handle mint and burn
        mint: Some(MinterData {
            minter: deps.api.canonical_address(&env.contract.address)?,
            cap: None,
        }),
    };
    token_info(deps.storage).save(&data)?;

    let places = DecimalPlaces::new(msg.decimals, msg.reserve_decimals);
    let supply = CurveState::new(msg.reserve_denom, places);
    CURVE_STATE.save(deps.storage, &supply)?;

    CURVE_TYPE.save(deps.storage, &msg.curve_type)?;

    Ok(Response::default())
}

pub fn handle(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: HandleMsg,
) -> Result<Response, ContractError> {
    // default implementation stores curve info as enum, you can do something else in a derived
    // contract and just pass in your custom curve to do_handle
    let curve_type = CURVE_TYPE.load(deps.storage)?;
    let curve_fn = curve_type.to_curve_fn();
    do_handle(deps, env, info, msg, curve_fn)
}

/// We pull out logic here, so we can import this from another contract and set a different Curve.
/// This contacts sets a curve with an enum in InitMsg and stored in state, but you may want to
/// use custom math not included - make this easily reusable
pub fn do_handle(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: HandleMsg,
    curve_fn: CurveFn,
) -> Result<Response, ContractError> {
    match msg {
        HandleMsg::Buy {} => handle_buy(deps, env, info, curve_fn),

        // we override these from cw20
        HandleMsg::Burn { amount } => Ok(handle_sell(deps, env, info, curve_fn, amount)?),
        HandleMsg::BurnFrom { owner, amount } => {
            Ok(handle_sell_from(deps, env, info, curve_fn, owner, amount)?)
        }

        // these all come from cw20-base to implement the cw20 standard
        HandleMsg::Transfer { recipient, amount } => {
            Ok(handle_transfer(deps, env, info, recipient, amount)?)
        }
        HandleMsg::Send {
            contract,
            amount,
            msg,
        } => Ok(handle_send(deps, env, info, contract, amount, msg)?),
        HandleMsg::IncreaseAllowance {
            spender,
            amount,
            expires,
        } => Ok(handle_increase_allowance(
            deps, env, info, spender, amount, expires,
        )?),
        HandleMsg::DecreaseAllowance {
            spender,
            amount,
            expires,
        } => Ok(handle_decrease_allowance(
            deps, env, info, spender, amount, expires,
        )?),
        HandleMsg::TransferFrom {
            owner,
            recipient,
            amount,
        } => Ok(handle_transfer_from(
            deps, env, info, owner, recipient, amount,
        )?),
        HandleMsg::SendFrom {
            owner,
            contract,
            amount,
            msg,
        } => Ok(handle_send_from(
            deps, env, info, owner, contract, amount, msg,
        )?),
    }
}

pub fn handle_buy(
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
    let minted = (new_supply - state.supply)?;
    state.supply = new_supply;
    CURVE_STATE.save(deps.storage, &state)?;

    // call into cw20-base to mint the token, call as self as no one else is allowed
    let sub_info = MessageInfo {
        sender: env.contract.address.clone(),
        funds: vec![],
    };
    handle_mint(deps, env, sub_info, info.sender.clone(), minted)?;

    // bond them to the validator
    let res = Response {
        submessages: vec![],
        messages: vec![],
        attributes: vec![
            attr("action", "buy"),
            attr("from", info.sender),
            attr("reserve", payment),
            attr("supply", minted),
        ],
        data: None,
    };
    Ok(res)
}

pub fn handle_sell(
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

pub fn handle_sell_from(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    curve_fn: CurveFn,
    owner: HumanAddr,
    amount: Uint128,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    let owner_raw = deps.api.canonical_address(&owner)?;
    let spender_raw = deps.api.canonical_address(&info.sender)?;

    // deduct allowance before doing anything else have enough allowance
    deduct_allowance(deps.storage, &owner_raw, &spender_raw, &env.block, amount)?;

    // do all the work in do_sell
    let receiver = info.sender;
    let owner_info = MessageInfo {
        sender: owner,
        funds: info.funds,
    };
    let mut res = do_sell(deps, env, owner_info, curve_fn, receiver.clone(), amount)?;

    // add our custom attributes
    res.attributes.push(attr("action", "burn_from"));
    res.attributes.push(attr("by", receiver));
    Ok(res)
}

fn do_sell(
    mut deps: DepsMut,
    env: Env,
    // info.sender is the one burning tokens
    info: MessageInfo,
    curve_fn: CurveFn,
    // receiver is the one who gains (same for handle_sell, diff for handle_sell_from)
    receiver: HumanAddr,
    amount: Uint128,
) -> Result<Response, ContractError> {
    // burn from the caller, this ensures there are tokens to cover this
    handle_burn(deps.branch(), env.clone(), info.clone(), amount)?;

    // calculate how many tokens can be purchased with this and mint them
    let mut state = CURVE_STATE.load(deps.storage)?;
    let curve = curve_fn(state.decimals);
    state.supply = (state.supply - amount)?;
    let new_reserve = curve.reserve(state.supply);
    let released = (state.reserve - new_reserve)?;
    state.reserve = new_reserve;
    CURVE_STATE.save(deps.storage, &state)?;

    // now send the tokens to the sender (TODO: for sell_from we do something else, right???)
    let msg = BankMsg::Send {
        to_address: receiver,
        amount: coins(released.u128(), state.reserve_denom),
    };
    let res = Response {
        submessages: vec![],
        messages: vec![msg.into()],
        attributes: vec![
            attr("from", info.sender),
            attr("supply", amount),
            attr("reserve", released),
        ],
        data: None,
    };
    Ok(res)
}

pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    // default implementation stores curve info as enum, you can do something else in a derived
    // contract and just pass in your custom curve to do_handle
    let curve_type = CURVE_TYPE.load(deps.storage)?;
    let curve_fn = curve_type.to_curve_fn();
    do_query(deps, env, msg, curve_fn)
}

/// We pull out logic here, so we can import this from another contract and set a different Curve.
/// This contacts sets a curve with an enum in InitMsg and stored in state, but you may want to
/// use custom math not included - make this easily reusable
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

    // This we can get from the local digits stored in init
    let curve = curve_fn(decimals);
    let spot_price = curve.spot_price(supply);

    Ok(CurveInfoResponse {
        reserve,
        supply,
        reserve_denom,
        spot_price,
    })
}

// this is poor mans "skip" flag
#[cfg(test)]
mod tests {
    use super::*;
    use crate::msg::CurveType;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coin, Decimal};
    use cw0::PaymentError;

    const DENOM: &str = "satoshi";
    const CREATOR: &str = "creator";
    const INVESTOR: &str = "investor";
    const BUYER: &str = "buyer";

    fn default_init(decimals: u8, reserve_decimals: u8, curve_type: CurveType) -> InitMsg {
        InitMsg {
            name: "Bonded".to_string(),
            symbol: "EPOXY".to_string(),
            decimals,
            reserve_denom: DENOM.to_string(),
            reserve_decimals,
            curve_type,
        }
    }

    fn get_balance<U: Into<HumanAddr>>(deps: Deps, addr: U) -> Uint128 {
        query_balance(deps, addr.into()).unwrap().balance
    }

    fn setup_test(deps: DepsMut, decimals: u8, reserve_decimals: u8, curve_type: CurveType) {
        // this matches `linear_curve` test case from curves.rs
        let creator = HumanAddr::from(CREATOR);
        let msg = default_init(decimals, reserve_decimals, curve_type.clone());
        let info = mock_info(&creator, &[]);

        // make sure we can init with this
        let res = init(deps, mock_env(), info, msg.clone()).unwrap();
        assert_eq!(0, res.messages.len());
    }

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(&[]);

        // this matches `linear_curve` test case from curves.rs
        let creator = HumanAddr::from("creator");
        let curve_type = CurveType::SquareRoot {
            slope: Uint128(1),
            scale: 1,
        };
        let msg = default_init(2, 8, curve_type.clone());
        let info = mock_info(&creator, &[]);

        // make sure we can init with this
        let res = init(deps.as_mut(), mock_env(), info, msg.clone()).unwrap();
        assert_eq!(0, res.messages.len());

        // token info is proper
        let token = query_token_info(deps.as_ref()).unwrap();
        assert_eq!(&token.name, &msg.name);
        assert_eq!(&token.symbol, &msg.symbol);
        assert_eq!(token.decimals, 2);
        assert_eq!(token.total_supply, Uint128(0));

        // curve state is sensible
        let state = query_curve_info(deps.as_ref(), curve_type.to_curve_fn()).unwrap();
        assert_eq!(state.reserve, Uint128(0));
        assert_eq!(state.supply, Uint128(0));
        assert_eq!(state.reserve_denom.as_str(), DENOM);
        // spot price 0 as supply is 0
        assert_eq!(state.spot_price, Decimal::zero());

        // curve type is stored properly
        let curve = CURVE_TYPE.load(&mut deps.storage).unwrap();
        assert_eq!(curve_type, curve);

        // no balance
        assert_eq!(get_balance(deps.as_ref(), &creator), Uint128(0));
    }

    #[test]
    fn buy_issues_tokens() {
        let mut deps = mock_dependencies(&[]);
        let curve_type = CurveType::Linear {
            slope: Uint128(1),
            scale: 1,
        };
        setup_test(deps.as_mut(), 2, 8, curve_type.clone());

        // succeeds with proper token (5 BTC = 5*10^8 satoshi)
        let info = mock_info(INVESTOR, &coins(500_000_000, DENOM));
        let buy = HandleMsg::Buy {};
        handle(deps.as_mut(), mock_env(), info, buy.clone()).unwrap();

        // bob got 1000 EPOXY (10.00)
        assert_eq!(get_balance(deps.as_ref(), INVESTOR), Uint128(1000));
        assert_eq!(get_balance(deps.as_ref(), BUYER), Uint128(0));

        // send them all to buyer
        let info = mock_info(INVESTOR, &[]);
        let send = HandleMsg::Transfer {
            recipient: BUYER.into(),
            amount: Uint128(1000),
        };
        handle(deps.as_mut(), mock_env(), info, send).unwrap();

        // ensure balances updated
        assert_eq!(get_balance(deps.as_ref(), INVESTOR), Uint128(0));
        assert_eq!(get_balance(deps.as_ref(), BUYER), Uint128(1000));

        // second stake needs more to get next 1000 EPOXY
        let info = mock_info(INVESTOR, &coins(1_500_000_000, DENOM));
        handle(deps.as_mut(), mock_env(), info, buy.clone()).unwrap();

        // ensure balances updated
        assert_eq!(get_balance(deps.as_ref(), INVESTOR), Uint128(1000));
        assert_eq!(get_balance(deps.as_ref(), BUYER), Uint128(1000));

        // check curve info updated
        let curve = query_curve_info(deps.as_ref(), curve_type.to_curve_fn()).unwrap();
        assert_eq!(curve.reserve, Uint128(2_000_000_000));
        assert_eq!(curve.supply, Uint128(2000));
        assert_eq!(curve.spot_price, Decimal::percent(200));

        // check token info updated
        let token = query_token_info(deps.as_ref()).unwrap();
        assert_eq!(token.decimals, 2);
        assert_eq!(token.total_supply, Uint128(2000));
    }

    #[test]
    fn bonding_fails_with_wrong_denom() {
        let mut deps = mock_dependencies(&[]);
        let curve_type = CurveType::Linear {
            slope: Uint128(1),
            scale: 1,
        };
        setup_test(deps.as_mut(), 2, 8, curve_type.clone());

        // fails when no tokens sent
        let info = mock_info(INVESTOR, &[]);
        let buy = HandleMsg::Buy {};
        let err = handle(deps.as_mut(), mock_env(), info, buy.clone()).unwrap_err();
        assert_eq!(err, PaymentError::NoFunds {}.into());

        // fails when wrong tokens sent
        let info = mock_info(INVESTOR, &coins(1234567, "wei"));
        let err = handle(deps.as_mut(), mock_env(), info, buy.clone()).unwrap_err();
        assert_eq!(err, PaymentError::MissingDenom(DENOM.into()).into());

        // fails when too many tokens sent
        let info = mock_info(INVESTOR, &[coin(3400022, DENOM), coin(1234567, "wei")]);
        let err = handle(deps.as_mut(), mock_env(), info, buy.clone()).unwrap_err();
        assert_eq!(err, PaymentError::ExtraDenom("wei".to_string()).into());
    }

    #[test]
    fn burning_sends_reserve() {
        let mut deps = mock_dependencies(&[]);
        let curve_type = CurveType::Linear {
            slope: Uint128(1),
            scale: 1,
        };
        setup_test(deps.as_mut(), 2, 8, curve_type.clone());

        // succeeds with proper token (20 BTC = 20*10^8 satoshi)
        let info = mock_info(INVESTOR, &coins(2_000_000_000, DENOM));
        let buy = HandleMsg::Buy {};
        handle(deps.as_mut(), mock_env(), info, buy).unwrap();

        // bob got 2000 EPOXY (20.00)
        assert_eq!(get_balance(deps.as_ref(), INVESTOR), Uint128(2000));

        // cannot burn too much
        let info = mock_info(INVESTOR, &[]);
        let burn = HandleMsg::Burn {
            amount: Uint128(3000),
        };
        let err = handle(deps.as_mut(), mock_env(), info, burn).unwrap_err();
        assert_eq!("Cannot subtract 3000 from 2000", err.to_string().as_str());

        // burn 1000 EPOXY to get back 15BTC (*10^8)
        let info = mock_info(INVESTOR, &[]);
        let burn = HandleMsg::Burn {
            amount: Uint128(1000),
        };
        let res = handle(deps.as_mut(), mock_env(), info, burn).unwrap();

        // balance is lower
        assert_eq!(get_balance(deps.as_ref(), INVESTOR), Uint128(1000));

        // ensure we got our money back
        assert_eq!(1, res.messages.len());
        assert_eq!(
            &res.messages[0],
            &BankMsg::Send {
                to_address: INVESTOR.into(),
                amount: coins(1_500_000_000, DENOM),
            }
            .into()
        );

        // check curve info updated
        let curve = query_curve_info(deps.as_ref(), curve_type.to_curve_fn()).unwrap();
        assert_eq!(curve.reserve, Uint128(500_000_000));
        assert_eq!(curve.supply, Uint128(1000));
        assert_eq!(curve.spot_price, Decimal::percent(100));

        // check token info updated
        let token = query_token_info(deps.as_ref()).unwrap();
        assert_eq!(token.decimals, 2);
        assert_eq!(token.total_supply, Uint128(1000));
    }

    #[test]
    fn cw20_imports_work() {
        let mut deps = mock_dependencies(&[]);
        let curve_type = CurveType::Constant {
            value: Uint128(15),
            scale: 1,
        };
        setup_test(deps.as_mut(), 9, 6, curve_type.clone());

        let alice: &str = "alice";
        let bob: &str = "bobby";
        let carl: &str = "carl";

        // spend 45_000 uatom for 30_000_000 EPOXY
        let info = mock_info(bob, &coins(45_000, DENOM));
        let buy = HandleMsg::Buy {};
        handle(deps.as_mut(), mock_env(), info, buy.clone()).unwrap();

        // check balances
        assert_eq!(get_balance(deps.as_ref(), bob), Uint128(30_000_000));
        assert_eq!(get_balance(deps.as_ref(), carl), Uint128(0));

        // send coins to carl
        let bob_info = mock_info(bob, &[]);
        let transfer = HandleMsg::Transfer {
            recipient: carl.into(),
            amount: Uint128(2_000_000),
        };
        handle(deps.as_mut(), mock_env(), bob_info.clone(), transfer).unwrap();
        assert_eq!(get_balance(deps.as_ref(), bob), Uint128(28_000_000));
        assert_eq!(get_balance(deps.as_ref(), carl), Uint128(2_000_000));

        // allow alice
        let allow = HandleMsg::IncreaseAllowance {
            spender: alice.into(),
            amount: Uint128(35_000_000),
            expires: None,
        };
        handle(deps.as_mut(), mock_env(), bob_info.clone(), allow).unwrap();
        assert_eq!(get_balance(deps.as_ref(), bob), Uint128(28_000_000));
        assert_eq!(get_balance(deps.as_ref(), alice), Uint128(0));
        assert_eq!(
            query_allowance(deps.as_ref(), bob.into(), alice.into())
                .unwrap()
                .allowance,
            Uint128(35_000_000)
        );

        // alice takes some for herself
        let self_pay = HandleMsg::TransferFrom {
            owner: bob.into(),
            recipient: alice.into(),
            amount: Uint128(25_000_000),
        };
        let alice_info = mock_info(alice, &[]);
        handle(deps.as_mut(), mock_env(), alice_info.clone(), self_pay).unwrap();
        assert_eq!(get_balance(deps.as_ref(), bob), Uint128(3_000_000));
        assert_eq!(get_balance(deps.as_ref(), alice), Uint128(25_000_000));
        assert_eq!(get_balance(deps.as_ref(), carl), Uint128(2_000_000));
        assert_eq!(
            query_allowance(deps.as_ref(), bob.into(), alice.into())
                .unwrap()
                .allowance,
            Uint128(10_000_000)
        );

        // test burn from works properly (burn tested in burning_sends_reserve)
        // cannot burn more than they have

        let info = mock_info(alice, &[]);
        let burn_from = HandleMsg::BurnFrom {
            owner: bob.into(),
            amount: Uint128(3_300_000),
        };
        let err = handle(deps.as_mut(), mock_env(), info, burn_from).unwrap_err();
        assert_eq!(
            "Cannot subtract 3300000 from 3000000",
            err.to_string().as_str()
        );

        // burn 1_000_000 EPOXY to get back 1_500 DENOM (constant curve)
        let info = mock_info(alice, &[]);
        let burn_from = HandleMsg::BurnFrom {
            owner: bob.into(),
            amount: Uint128(1_000_000),
        };
        let res = handle(deps.as_mut(), mock_env(), info, burn_from).unwrap();

        // bob balance is lower, not alice
        assert_eq!(get_balance(deps.as_ref(), alice), Uint128(25_000_000));
        assert_eq!(get_balance(deps.as_ref(), bob), Uint128(2_000_000));

        // ensure alice got our money back
        assert_eq!(1, res.messages.len());
        assert_eq!(
            &res.messages[0],
            &BankMsg::Send {
                to_address: alice.into(),
                amount: coins(1_500, DENOM),
            }
            .into()
        );
    }
}
