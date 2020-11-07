#![cfg(test)]

use cosmwasm_std::testing::{mock_env, MockApi, MockStorage};
use cosmwasm_std::{coins, HumanAddr, Uint128};
use cw20::{Cw20CoinHuman, Cw20Contract};
use cw_multi_test::{Contract, ContractWrapper, Router, SimpleBank};

use crate::msg::InitMsg;

fn mock_router() -> Router {
    let env = mock_env();
    let api = Box::new(MockApi::default());
    let bank = SimpleBank {};

    Router::new(api, env.block, bank, || Box::new(MockStorage::new()))
}

pub fn contract_escrow() -> Box<dyn Contract> {
    let contract = ContractWrapper::new(
        crate::contract::handle,
        crate::contract::init,
        crate::contract::query,
    );
    Box::new(contract)
}

pub fn contract_cw20() -> Box<dyn Contract> {
    let contract = ContractWrapper::new(
        cw20_base::contract::handle,
        cw20_base::contract::init,
        cw20_base::contract::query,
    );
    Box::new(contract)
}

#[test]
fn reflect_send_cw20_tokens() {
    let mut router = mock_router();

    // set personal balance
    let owner = HumanAddr::from("owner");
    let init_funds = coins(2000, "btc");
    router
        .set_bank_balance(owner.clone(), init_funds.clone())
        .unwrap();

    // set up cw20 contract with some tokens
    let cw20_id = router.store_code(contract_cw20());
    let msg = cw20_base::msg::InitMsg {
        name: "Cash Money".to_string(),
        symbol: "CASH".to_string(),
        decimals: 2,
        initial_balances: vec![Cw20CoinHuman {
            address: owner.clone(),
            amount: Uint128(5000),
        }],
        mint: None,
    };
    let cash_addr = router
        .instantiate_contract(cw20_id, &owner, &msg, &[], "CASH")
        .unwrap();

    // set up reflect contract
    let escrow_id = router.store_code(contract_escrow());
    let escrow_addr = router
        .instantiate_contract(escrow_id, &owner, &InitMsg {}, &[], "Escrow")
        .unwrap();

    // they are different
    assert_ne!(cash_addr, escrow_addr);

    // set up cw20 helpers
    let cash = Cw20Contract(cash_addr.clone());

    // ensure our balances
    let owner_balance = cash.balance(&router, owner.clone()).unwrap();
    assert_eq!(owner_balance, Uint128(5000));
    let escrow_balance = cash.balance(&router, escrow_addr.clone()).unwrap();
    assert_eq!(escrow_balance, Uint128(0));

    // TODO: send some tokens to create an escrow

    // TODO: ensure balances updated

    // TODO: ensure escrow properly created (use attributes?)
}
