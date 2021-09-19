#![cfg(test)]

use cosmwasm_std::{coins, to_binary, Addr, Empty, Uint128};
use cw20::{Cw20Coin, Cw20Contract, Cw20ExecuteMsg};
use cw_multi_test::{App, Contract, ContractWrapper, Executor};

use crate::msg::{CreateMsg, DetailsResponse, ExecuteMsg, InstantiateMsg, QueryMsg, ReceiveMsg};

pub fn contract_escrow() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    );
    Box::new(contract)
}

pub fn contract_cw20() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cw20_base::contract::execute,
        cw20_base::contract::instantiate,
        cw20_base::contract::query,
    );
    Box::new(contract)
}

#[test]
// receive cw20 tokens and release upon approval
fn escrow_happy_path_cw20_tokens() {
    // set personal balance
    let owner = Addr::unchecked("owner");
    let init_funds = coins(2000, "btc");

    let mut router = App::new(|router, _, storage| {
        router
            .bank
            .init_balance(storage, &owner, init_funds)
            .unwrap();
    });

    // set up cw20 contract with some tokens
    let cw20_id = router.store_code(contract_cw20());
    let msg = cw20_base::msg::InstantiateMsg {
        name: "Cash Money".to_string(),
        symbol: "CASH".to_string(),
        decimals: 2,
        initial_balances: vec![Cw20Coin {
            address: owner.to_string(),
            amount: Uint128::new(5000),
        }],
        mint: None,
        marketing: None,
    };
    let cash_addr = router
        .instantiate_contract(cw20_id, owner.clone(), &msg, &[], "CASH", None)
        .unwrap();

    // set up reflect contract
    let escrow_id = router.store_code(contract_escrow());
    let escrow_addr = router
        .instantiate_contract(
            escrow_id,
            owner.clone(),
            &InstantiateMsg {},
            &[],
            "Escrow",
            None,
        )
        .unwrap();

    // they are different
    assert_ne!(cash_addr, escrow_addr);

    // set up cw20 helpers
    let cash = Cw20Contract(cash_addr.clone());

    // ensure our balances
    let owner_balance = cash.balance(&router, owner.clone()).unwrap();
    assert_eq!(owner_balance, Uint128::new(5000));
    let escrow_balance = cash.balance(&router, escrow_addr.clone()).unwrap();
    assert_eq!(escrow_balance, Uint128::zero());

    // send some tokens to create an escrow
    let arb = Addr::unchecked("arbiter");
    let ben = String::from("beneficiary");
    let id = "demo".to_string();
    let create_msg = ReceiveMsg::Create(CreateMsg {
        id: id.clone(),
        arbiter: arb.to_string(),
        recipient: ben.clone(),
        end_height: None,
        end_time: None,
        cw20_whitelist: None,
    });
    let send_msg = Cw20ExecuteMsg::Send {
        contract: escrow_addr.to_string(),
        amount: Uint128::new(1200),
        msg: to_binary(&create_msg).unwrap(),
    };
    let res = router
        .execute_contract(owner.clone(), cash_addr.clone(), &send_msg, &[])
        .unwrap();
    assert_eq!(4, res.events.len());
    println!("{:?}", res.events);

    assert_eq!(res.events[0].ty.as_str(), "execute");
    let cw20_attr = res.custom_attrs(1);
    println!("{:?}", cw20_attr);
    assert_eq!(4, cw20_attr.len());

    assert_eq!(res.events[2].ty.as_str(), "execute");
    let escrow_attr = res.custom_attrs(3);
    println!("{:?}", escrow_attr);
    assert_eq!(2, escrow_attr.len());

    // ensure balances updated
    let owner_balance = cash.balance(&router, owner.clone()).unwrap();
    assert_eq!(owner_balance, Uint128::new(3800));
    let escrow_balance = cash.balance(&router, escrow_addr.clone()).unwrap();
    assert_eq!(escrow_balance, Uint128::new(1200));

    // ensure escrow properly created
    let details: DetailsResponse = router
        .wrap()
        .query_wasm_smart(&escrow_addr, &QueryMsg::Details { id: id.clone() })
        .unwrap();
    assert_eq!(id, details.id);
    assert_eq!(arb, details.arbiter);
    assert_eq!(ben, details.recipient);
    assert_eq!(
        vec![Cw20Coin {
            address: cash_addr.to_string(),
            amount: Uint128::new(1200)
        }],
        details.cw20_balance
    );

    // release escrow
    let approve_msg = ExecuteMsg::Approve { id };
    let _ = router
        .execute_contract(arb, escrow_addr.clone(), &approve_msg, &[])
        .unwrap();

    // ensure balances updated - release to ben
    let owner_balance = cash.balance(&router, owner).unwrap();
    assert_eq!(owner_balance, Uint128::new(3800));
    let escrow_balance = cash.balance(&router, escrow_addr).unwrap();
    assert_eq!(escrow_balance, Uint128::zero());
    let ben_balance = cash.balance(&router, ben).unwrap();
    assert_eq!(ben_balance, Uint128::new(1200));
}
