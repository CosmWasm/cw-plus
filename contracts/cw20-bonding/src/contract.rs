use cosmwasm_std::{
    attr, coins, to_binary, BankMsg, Binary, Deps, DepsMut, Env, HandleResponse, HumanAddr,
    InitResponse, MessageInfo, StdResult, Uint128,
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

use crate::curves::{decimal, Constant, Curve, DecimalPlaces};
use crate::error::ContractError;
use crate::msg::{CurveInfoResponse, HandleMsg, InitMsg, QueryMsg};
use crate::state::{CurveState, CURVE_STATE};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cw20-bonding";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn init(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InitMsg,
) -> Result<InitResponse, ContractError> {
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

    let supply = CurveState::new(msg.reserve_denom);
    CURVE_STATE.save(deps.storage, &supply)?;

    Ok(InitResponse::default())
}

pub fn handle(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: HandleMsg,
) -> Result<HandleResponse, ContractError> {
    // TODO: we need to get these values from somewhere, based on each token's definition
    // (Pass in via InitMsg?)
    const NORMALIZE: DecimalPlaces = DecimalPlaces {
        supply: 6,
        reserve: 6,
    };

    // TODO: where does this come from in real code? (same in handle and query)
    // right now test with 2 reserve to buy 1 supply
    let curve = Constant::new(decimal(2u128, 0), NORMALIZE);

    match msg {
        HandleMsg::Buy {} => handle_buy(deps, env, info, &curve),

        // we override these from cw20
        HandleMsg::Burn { amount } => Ok(handle_sell(deps, env, info, &curve, amount)?),
        HandleMsg::BurnFrom { owner, amount } => {
            Ok(handle_sell_from(deps, env, info, &curve, owner, amount)?)
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
    curve: &dyn Curve,
) -> Result<HandleResponse, ContractError> {
    let mut state = CURVE_STATE.load(deps.storage)?;

    // ensure the sent denom was proper
    let payment = match info.sent_funds.len() {
        0 => Err(ContractError::NoFunds {}),
        1 => {
            if info.sent_funds[0].denom == state.reserve_denom {
                Ok(info.sent_funds[0].amount)
            } else {
                Err(ContractError::MissingDenom(state.reserve_denom.clone()))
            }
        }
        _ => Err(ContractError::ExtraDenoms(state.reserve_denom.clone())),
    }?;
    if payment.is_zero() {
        return Err(ContractError::NoFunds {});
    }

    // calculate how many tokens can be purchased with this and mint them
    state.reserve += payment;
    let new_supply = curve.supply(state.reserve);
    let minted = (new_supply - state.supply)?;
    state.supply = new_supply;
    CURVE_STATE.save(deps.storage, &state)?;

    // call into cw20-base to mint the token, call as self as no one else is allowed
    let sub_info = MessageInfo {
        sender: env.contract.address.clone(),
        sent_funds: vec![],
    };
    handle_mint(deps, env, sub_info, info.sender.clone(), minted)?;

    // bond them to the validator
    let res = HandleResponse {
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
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    curve: &dyn Curve,
    amount: Uint128,
) -> Result<HandleResponse, ContractError> {
    // burn from the caller, this ensures there are tokens to cover this
    handle_burn(deps.branch(), env.clone(), info.clone(), amount)?;

    // calculate how many tokens can be purchased with this and mint them
    let mut state = CURVE_STATE.load(deps.storage)?;
    state.supply = (state.supply - amount)?;
    let new_reserve = curve.reserve(state.supply);
    let released = (state.reserve - new_reserve)?;
    state.reserve = new_reserve;
    CURVE_STATE.save(deps.storage, &state)?;

    // now send the tokens to the sender (TODO: for sell_from we do something else, right???)
    let msg = BankMsg::Send {
        from_address: env.contract.address,
        to_address: info.sender.clone(),
        amount: coins(released.u128(), state.reserve_denom),
    };
    let res = HandleResponse {
        messages: vec![msg.into()],
        attributes: vec![
            attr("action", "sell"),
            attr("from", info.sender),
            attr("supply", amount),
            attr("reserve", released),
        ],
        data: None,
    };
    Ok(res)
}

pub fn handle_sell_from(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    curve: &dyn Curve,
    owner: HumanAddr,
    amount: Uint128,
) -> Result<HandleResponse, ContractError> {
    let owner_raw = deps.api.canonical_address(&owner)?;
    let spender_raw = deps.api.canonical_address(&info.sender)?;

    // deduct allowance before doing anything else have enough allowance
    deduct_allowance(deps.storage, &owner_raw, &spender_raw, &env.block, amount)?;

    // TODO: don't return verbatim, different return attrs
    let owner_info = MessageInfo {
        sender: owner,
        sent_funds: info.sent_funds,
    };
    handle_sell(deps, env, owner_info, curve, amount)
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    // TODO: we need to get these values from somewhere, based on each token's definition
    // (Pass in via InitMsg?)
    const NORMALIZE: DecimalPlaces = DecimalPlaces {
        supply: 6,
        reserve: 6,
    };

    // TODO: where does this come from in real code? (same in handle and query)
    // right now test with 2 reserve to buy 1 supply
    let curve = Constant::new(decimal(2u128, 0), NORMALIZE);

    match msg {
        // custom queries
        QueryMsg::CurveInfo {} => to_binary(&query_curve_info(deps, &curve)?),
        // inherited from cw20-base
        QueryMsg::TokenInfo {} => to_binary(&query_token_info(deps)?),
        QueryMsg::Balance { address } => to_binary(&query_balance(deps, address)?),
        QueryMsg::Allowance { owner, spender } => {
            to_binary(&query_allowance(deps, owner, spender)?)
        }
    }
}

pub fn query_curve_info(deps: Deps, curve: &dyn Curve) -> StdResult<CurveInfoResponse> {
    let CurveState {
        reserve,
        supply,
        reserve_denom,
    } = CURVE_STATE.load(deps.storage)?;

    // This we can get from the local digits stored in init
    let spot_price = curve.spot_price(supply);

    Ok(CurveInfoResponse {
        reserve,
        supply,
        reserve_denom,
        spot_price,
    })
}

// this is poor mans "skip" flag
#[cfg(target_arch = "arm")]
#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{
        mock_dependencies, mock_env, mock_info, MockQuerier, MOCK_CONTRACT_ADDR,
    };
    use cosmwasm_std::{coins, Coin, CosmosMsg, Decimal, FullDelegation, Validator};
    use cw0::{claim::Claim, Duration, DAY, HOUR, WEEK};
    use std::str::FromStr;

    fn sample_validator<U: Into<HumanAddr>>(addr: U) -> Validator {
        Validator {
            address: addr.into(),
            commission: Decimal::percent(3),
            max_commission: Decimal::percent(10),
            max_change_rate: Decimal::percent(1),
        }
    }

    fn sample_delegation<U: Into<HumanAddr>>(addr: U, amount: Coin) -> FullDelegation {
        let can_redelegate = amount.clone();
        let accumulated_rewards = coins(0, &amount.denom);
        FullDelegation {
            validator: addr.into(),
            delegator: HumanAddr::from(MOCK_CONTRACT_ADDR),
            amount,
            can_redelegate,
            accumulated_rewards,
        }
    }

    fn set_validator(querier: &mut MockQuerier) {
        querier.update_staking("ustake", &[sample_validator(DEFAULT_VALIDATOR)], &[]);
    }

    fn set_delegation(querier: &mut MockQuerier, amount: u128, denom: &str) {
        querier.update_staking(
            "ustake",
            &[sample_validator(DEFAULT_VALIDATOR)],
            &[sample_delegation(DEFAULT_VALIDATOR, coin(amount, denom))],
        );
    }

    // just a test helper, forgive the panic
    fn later(env: &Env, delta: Duration) -> Env {
        let time_delta = match delta {
            Duration::Time(t) => t,
            _ => panic!("Must provide duration in time"),
        };
        let mut res = env.clone();
        res.block.time += time_delta;
        res
    }

    const DEFAULT_VALIDATOR: &str = "default-validator";

    fn default_init(tax_percent: u64, min_withdrawal: u128) -> InitMsg {
        InitMsg {
            name: "Cool Derivative".to_string(),
            symbol: "DRV".to_string(),
            decimals: 9,
            validator: HumanAddr::from(DEFAULT_VALIDATOR),
            unbonding_period: DAY * 3,
            exit_tax: Decimal::percent(tax_percent),
            min_withdrawal: Uint128(min_withdrawal),
        }
    }

    fn get_balance<U: Into<HumanAddr>>(deps: Deps, addr: U) -> Uint128 {
        query_balance(deps, addr.into()).unwrap().balance
    }

    fn get_claims<U: Into<HumanAddr>>(deps: Deps, addr: U) -> Vec<Claim> {
        query_claims(deps, addr.into()).unwrap().claims
    }

    #[test]
    fn initialization_with_missing_validator() {
        let mut deps = mock_dependencies(&[]);
        deps.querier
            .update_staking("ustake", &[sample_validator("john")], &[]);

        let creator = HumanAddr::from("creator");
        let msg = InitMsg {
            name: "Cool Derivative".to_string(),
            symbol: "DRV".to_string(),
            decimals: 9,
            validator: HumanAddr::from("my-validator"),
            unbonding_period: WEEK,
            exit_tax: Decimal::percent(2),
            min_withdrawal: Uint128(50),
        };
        let info = mock_info(&creator, &[]);

        // make sure we can init with this
        let res = init(deps.as_mut(), mock_env(), info, msg.clone());
        match res.unwrap_err() {
            ContractError::NotInValidatorSet { .. } => {}
            _ => panic!("expected unregistered validator error"),
        }
    }

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(&[]);
        deps.querier.update_staking(
            "ustake",
            &[
                sample_validator("john"),
                sample_validator("mary"),
                sample_validator("my-validator"),
            ],
            &[],
        );

        let creator = HumanAddr::from("creator");
        let msg = InitMsg {
            name: "Cool Derivative".to_string(),
            symbol: "DRV".to_string(),
            decimals: 0,
            validator: HumanAddr::from("my-validator"),
            unbonding_period: HOUR * 12,
            exit_tax: Decimal::percent(2),
            min_withdrawal: Uint128(50),
        };
        let info = mock_info(&creator, &[]);

        // make sure we can init with this
        let res = init(deps.as_mut(), mock_env(), info, msg.clone()).unwrap();
        assert_eq!(0, res.messages.len());

        // token info is proper
        let token = query_token_info(deps.as_ref()).unwrap();
        assert_eq!(&token.name, &msg.name);
        assert_eq!(&token.symbol, &msg.symbol);
        assert_eq!(token.decimals, msg.decimals);
        assert_eq!(token.total_supply, Uint128(0));

        // no balance
        assert_eq!(get_balance(deps.as_ref(), &creator), Uint128(0));
        // no claims
        assert_eq!(get_claims(deps.as_ref(), &creator), vec![]);

        // investment info correct
        let invest = query_investment(deps.as_ref()).unwrap();
        assert_eq!(&invest.owner, &creator);
        assert_eq!(&invest.validator, &msg.validator);
        assert_eq!(invest.exit_tax, msg.exit_tax);
        assert_eq!(invest.min_withdrawal, msg.min_withdrawal);

        assert_eq!(invest.token_supply, Uint128(0));
        assert_eq!(invest.staked_tokens, coin(0, "ustake"));
        assert_eq!(invest.nominal_value, Decimal::one());
    }

    #[test]
    fn bonding_issues_tokens() {
        let mut deps = mock_dependencies(&[]);
        set_validator(&mut deps.querier);

        let creator = HumanAddr::from("creator");
        let init_msg = default_init(2, 50);
        let info = mock_info(&creator, &[]);

        // make sure we can init with this
        let res = init(deps.as_mut(), mock_env(), info, init_msg).unwrap();
        assert_eq!(0, res.messages.len());

        // let's bond some tokens now
        let bob = HumanAddr::from("bob");
        let bond_msg = HandleMsg::Bond {};
        let info = mock_info(&bob, &[coin(10, "random"), coin(1000, "ustake")]);

        // try to bond and make sure we trigger delegation
        let res = handle(deps.as_mut(), mock_env(), info, bond_msg).unwrap();
        assert_eq!(1, res.messages.len());
        let delegate = &res.messages[0];
        match delegate {
            CosmosMsg::Staking(StakingMsg::Delegate { validator, amount }) => {
                assert_eq!(validator.as_str(), DEFAULT_VALIDATOR);
                assert_eq!(amount, &coin(1000, "ustake"));
            }
            _ => panic!("Unexpected message: {:?}", delegate),
        }

        // bob got 1000 DRV for 1000 stake at a 1.0 ratio
        assert_eq!(get_balance(deps.as_ref(), &bob), Uint128(1000));

        // investment info correct (updated supply)
        let invest = query_investment(deps.as_ref()).unwrap();
        assert_eq!(invest.token_supply, Uint128(1000));
        assert_eq!(invest.staked_tokens, coin(1000, "ustake"));
        assert_eq!(invest.nominal_value, Decimal::one());

        // token info also properly updated
        let token = query_token_info(deps.as_ref()).unwrap();
        assert_eq!(token.total_supply, Uint128(1000));
    }

    #[test]
    fn rebonding_changes_pricing() {
        let mut deps = mock_dependencies(&[]);
        set_validator(&mut deps.querier);

        let creator = HumanAddr::from("creator");
        let init_msg = default_init(2, 50);
        let info = mock_info(&creator, &[]);

        // make sure we can init with this
        let res = init(deps.as_mut(), mock_env(), info, init_msg).unwrap();
        assert_eq!(0, res.messages.len());

        // let's bond some tokens now
        let bob = HumanAddr::from("bob");
        let bond_msg = HandleMsg::Bond {};
        let info = mock_info(&bob, &[coin(10, "random"), coin(1000, "ustake")]);
        let res = handle(deps.as_mut(), mock_env(), info, bond_msg).unwrap();
        assert_eq!(1, res.messages.len());

        // update the querier with new bond
        set_delegation(&mut deps.querier, 1000, "ustake");

        // fake a reinvestment (this must be sent by the contract itself)
        let rebond_msg = HandleMsg::_BondAllTokens {};
        let info = mock_info(MOCK_CONTRACT_ADDR, &[]);
        deps.querier
            .update_balance(MOCK_CONTRACT_ADDR, coins(500, "ustake"));
        let _ = handle(deps.as_mut(), mock_env(), info, rebond_msg).unwrap();

        // update the querier with new bond
        set_delegation(&mut deps.querier, 1500, "ustake");

        // we should now see 1000 issues and 1500 bonded (and a price of 1.5)
        let invest = query_investment(deps.as_ref()).unwrap();
        assert_eq!(invest.token_supply, Uint128(1000));
        assert_eq!(invest.staked_tokens, coin(1500, "ustake"));
        let ratio = Decimal::from_str("1.5").unwrap();
        assert_eq!(invest.nominal_value, ratio);

        // we bond some other tokens and get a different issuance price (maintaining the ratio)
        let alice = HumanAddr::from("alice");
        let bond_msg = HandleMsg::Bond {};
        let info = mock_info(&alice, &[coin(3000, "ustake")]);
        let res = handle(deps.as_mut(), mock_env(), info, bond_msg).unwrap();
        assert_eq!(1, res.messages.len());

        // update the querier with new bond
        set_delegation(&mut deps.querier, 3000, "ustake");

        // alice should have gotten 2000 DRV for the 3000 stake, keeping the ratio at 1.5
        assert_eq!(get_balance(deps.as_ref(), &alice), Uint128(2000));

        let invest = query_investment(deps.as_ref()).unwrap();
        assert_eq!(invest.token_supply, Uint128(3000));
        assert_eq!(invest.staked_tokens, coin(4500, "ustake"));
        assert_eq!(invest.nominal_value, ratio);
    }

    #[test]
    fn bonding_fails_with_wrong_denom() {
        let mut deps = mock_dependencies(&[]);
        set_validator(&mut deps.querier);

        let creator = HumanAddr::from("creator");
        let init_msg = default_init(2, 50);
        let info = mock_info(&creator, &[]);

        // make sure we can init with this
        let res = init(deps.as_mut(), mock_env(), info, init_msg).unwrap();
        assert_eq!(0, res.messages.len());

        // let's bond some tokens now
        let bob = HumanAddr::from("bob");
        let bond_msg = HandleMsg::Bond {};
        let info = mock_info(&bob, &[coin(500, "photon")]);

        // try to bond and make sure we trigger delegation
        let res = handle(deps.as_mut(), mock_env(), info, bond_msg);
        match res.unwrap_err() {
            ContractError::EmptyBalance { .. } => {}
            e => panic!("Expected wrong denom error, got: {:?}", e),
        };
    }

    #[test]
    fn unbonding_maintains_price_ratio() {
        let mut deps = mock_dependencies(&[]);
        set_validator(&mut deps.querier);

        let creator = HumanAddr::from("creator");
        let init_msg = default_init(10, 50);
        let info = mock_info(&creator, &[]);

        // make sure we can init with this
        let res = init(deps.as_mut(), mock_env(), info, init_msg).unwrap();
        assert_eq!(0, res.messages.len());

        // let's bond some tokens now
        let bob = HumanAddr::from("bob");
        let bond_msg = HandleMsg::Bond {};
        let info = mock_info(&bob, &[coin(10, "random"), coin(1000, "ustake")]);
        let res = handle(deps.as_mut(), mock_env(), info, bond_msg).unwrap();
        assert_eq!(1, res.messages.len());

        // update the querier with new bond
        set_delegation(&mut deps.querier, 1000, "ustake");

        // fake a reinvestment (this must be sent by the contract itself)
        // after this, we see 1000 issues and 1500 bonded (and a price of 1.5)
        let rebond_msg = HandleMsg::_BondAllTokens {};
        let info = mock_info(MOCK_CONTRACT_ADDR, &[]);
        deps.querier
            .update_balance(MOCK_CONTRACT_ADDR, coins(500, "ustake"));
        let _ = handle(deps.as_mut(), mock_env(), info, rebond_msg).unwrap();

        // update the querier with new bond, lower balance
        set_delegation(&mut deps.querier, 1500, "ustake");
        deps.querier.update_balance(MOCK_CONTRACT_ADDR, vec![]);

        // creator now tries to unbond these tokens - this must fail
        let unbond_msg = HandleMsg::Unbond {
            amount: Uint128(600),
        };
        let info = mock_info(&creator, &[]);
        let res = handle(deps.as_mut(), mock_env(), info, unbond_msg);
        match res.unwrap_err() {
            ContractError::Std(StdError::Underflow { .. }) => {}
            e => panic!("unexpected error: {}", e),
        }

        // bob unbonds 600 tokens at 10% tax...
        // 60 are taken and send to the owner
        // 540 are unbonded in exchange for 540 * 1.5 = 810 native tokens
        let unbond_msg = HandleMsg::Unbond {
            amount: Uint128(600),
        };
        let owner_cut = Uint128(60);
        let bobs_claim = Uint128(810);
        let bobs_balance = Uint128(400);
        let env = mock_env();
        let info = mock_info(&bob, &[]);
        let res = handle(deps.as_mut(), env.clone(), info, unbond_msg).unwrap();
        assert_eq!(1, res.messages.len());
        let delegate = &res.messages[0];
        match delegate {
            CosmosMsg::Staking(StakingMsg::Undelegate { validator, amount }) => {
                assert_eq!(validator.as_str(), DEFAULT_VALIDATOR);
                assert_eq!(amount, &coin(bobs_claim.u128(), "ustake"));
            }
            _ => panic!("Unexpected message: {:?}", delegate),
        }

        // update the querier with new bond, lower balance
        set_delegation(&mut deps.querier, 690, "ustake");

        // check balances
        assert_eq!(get_balance(deps.as_ref(), &bob), bobs_balance);
        assert_eq!(get_balance(deps.as_ref(), &creator), owner_cut);
        // proper claims
        let expected_claims = vec![Claim {
            amount: bobs_claim,
            release_at: (DAY * 3).after(&env.block),
        }];
        assert_eq!(expected_claims, get_claims(deps.as_ref(), &bob));

        // supplies updated, ratio the same (1.5)
        let ratio = Decimal::from_str("1.5").unwrap();

        let invest = query_investment(deps.as_ref()).unwrap();
        assert_eq!(invest.token_supply, bobs_balance + owner_cut);
        assert_eq!(invest.staked_tokens, coin(690, "ustake")); // 1500 - 810
        assert_eq!(invest.nominal_value, ratio);
    }

    #[test]
    fn claims_paid_out_properly() {
        let mut deps = mock_dependencies(&[]);
        set_validator(&mut deps.querier);

        // create contract
        let creator = HumanAddr::from("creator");
        let init_msg = default_init(10, 50);
        let info = mock_info(&creator, &[]);
        init(deps.as_mut(), mock_env(), info, init_msg).unwrap();

        // bond some tokens
        let bob = HumanAddr::from("bob");
        let info = mock_info(&bob, &coins(1000, "ustake"));
        handle(deps.as_mut(), mock_env(), info, HandleMsg::Bond {}).unwrap();
        set_delegation(&mut deps.querier, 1000, "ustake");

        // unbond part of them
        let unbond_msg = HandleMsg::Unbond {
            amount: Uint128(600),
        };
        let env = mock_env();
        let info = mock_info(&bob, &[]);
        handle(deps.as_mut(), env.clone(), info.clone(), unbond_msg).unwrap();
        set_delegation(&mut deps.querier, 460, "ustake");

        // ensure claims are proper
        let bobs_claim = Uint128(540);
        let original_claims = vec![Claim {
            amount: bobs_claim,
            release_at: (DAY * 3).after(&env.block),
        }];
        assert_eq!(original_claims, get_claims(deps.as_ref(), &bob));

        // bob cannot exercise claims without enough balance
        let claim_ready = later(&env, (DAY * 3 + HOUR).unwrap());
        let too_soon = later(&env, DAY);
        let fail = handle(
            deps.as_mut(),
            claim_ready.clone(),
            info.clone(),
            HandleMsg::Claim {},
        );
        assert!(fail.is_err(), "{:?}", fail);

        // provide the balance, but claim not yet mature - also prohibited
        deps.querier
            .update_balance(MOCK_CONTRACT_ADDR, coins(540, "ustake"));
        let fail = handle(deps.as_mut(), too_soon, info.clone(), HandleMsg::Claim {});
        assert!(fail.is_err(), "{:?}", fail);

        // this should work with cash and claims ready
        let res = handle(
            deps.as_mut(),
            claim_ready,
            info.clone(),
            HandleMsg::Claim {},
        )
        .unwrap();
        assert_eq!(1, res.messages.len());
        let payout = &res.messages[0];
        match payout {
            CosmosMsg::Bank(BankMsg::Send {
                from_address,
                to_address,
                amount,
            }) => {
                assert_eq!(amount, &coins(540, "ustake"));
                assert_eq!(from_address.as_str(), MOCK_CONTRACT_ADDR);
                assert_eq!(to_address, &bob);
            }
            _ => panic!("Unexpected message: {:?}", payout),
        }

        // claims have been removed
        assert_eq!(get_claims(deps.as_ref(), &bob), vec![]);
    }

    #[test]
    fn cw20_imports_work() {
        let mut deps = mock_dependencies(&[]);
        set_validator(&mut deps.querier);

        // set the actors... bob stakes, sends coins to carl, and gives allowance to alice
        let bob = HumanAddr::from("bob");
        let alice = HumanAddr::from("alice");
        let carl = HumanAddr::from("carl");

        // create the contract
        let creator = HumanAddr::from("creator");
        let init_msg = default_init(2, 50);
        let info = mock_info(&creator, &[]);
        init(deps.as_mut(), mock_env(), info, init_msg).unwrap();

        // bond some tokens to create a balance
        let info = mock_info(&bob, &[coin(10, "random"), coin(1000, "ustake")]);
        handle(deps.as_mut(), mock_env(), info, HandleMsg::Bond {}).unwrap();

        // bob got 1000 DRV for 1000 stake at a 1.0 ratio
        assert_eq!(get_balance(deps.as_ref(), &bob), Uint128(1000));

        // send coins to carl
        let bob_info = mock_info(&bob, &[]);
        let transfer = HandleMsg::Transfer {
            recipient: carl.clone(),
            amount: Uint128(200),
        };
        handle(deps.as_mut(), mock_env(), bob_info.clone(), transfer).unwrap();
        assert_eq!(get_balance(deps.as_ref(), &bob), Uint128(800));
        assert_eq!(get_balance(deps.as_ref(), &carl), Uint128(200));

        // allow alice
        let allow = HandleMsg::IncreaseAllowance {
            spender: alice.clone(),
            amount: Uint128(350),
            expires: None,
        };
        handle(deps.as_mut(), mock_env(), bob_info.clone(), allow).unwrap();
        assert_eq!(get_balance(deps.as_ref(), &bob), Uint128(800));
        assert_eq!(get_balance(deps.as_ref(), &alice), Uint128(0));
        assert_eq!(
            query_allowance(deps.as_ref(), bob.clone(), alice.clone())
                .unwrap()
                .allowance,
            Uint128(350)
        );

        // alice takes some for herself
        let self_pay = HandleMsg::TransferFrom {
            owner: bob.clone(),
            recipient: alice.clone(),
            amount: Uint128(250),
        };
        let alice_info = mock_info(&alice, &[]);
        handle(deps.as_mut(), mock_env(), alice_info.clone(), self_pay).unwrap();
        assert_eq!(get_balance(deps.as_ref(), &bob), Uint128(550));
        assert_eq!(get_balance(deps.as_ref(), &alice), Uint128(250));
        assert_eq!(
            query_allowance(deps.as_ref(), bob.clone(), alice.clone())
                .unwrap()
                .allowance,
            Uint128(100)
        );

        // burn some, but not too much
        let burn_too_much = HandleMsg::Burn {
            amount: Uint128(1000),
        };
        let failed = handle(deps.as_mut(), mock_env(), bob_info.clone(), burn_too_much);
        assert!(failed.is_err());
        assert_eq!(get_balance(deps.as_ref(), &bob), Uint128(550));
        let burn = HandleMsg::Burn {
            amount: Uint128(130),
        };
        handle(deps.as_mut(), mock_env(), bob_info.clone(), burn).unwrap();
        assert_eq!(get_balance(deps.as_ref(), &bob), Uint128(420));
    }
}
