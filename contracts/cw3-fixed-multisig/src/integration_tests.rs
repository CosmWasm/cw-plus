#![cfg(test)]

use crate::contract::{handle, init, query};
use crate::msg::{HandleMsg, InitMsg, Voter};
use cosmwasm_std::testing::{mock_env, MockApi, MockStorage};
use cosmwasm_std::{from_binary, to_binary, HumanAddr, Uint128, WasmMsg, WasmQuery};
use cw0::Duration;
use cw20::{BalanceResponse, MinterResponse};
use cw20_base::msg::QueryMsg;
use cw3::Vote;
use cw_multi_test::{App, Contract, ContractWrapper, SimpleBank};

fn mock_app() -> App {
    let env = mock_env();
    let api = Box::new(MockApi::default());
    let bank = SimpleBank {};

    App::new(api, env.block, bank, || Box::new(MockStorage::new()))
}

pub fn contract_cw3_fixed_multisig() -> Box<dyn Contract> {
    let contract = ContractWrapper::new(handle, init, query);
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
// cw3 multisig account can control cw20 admin actions
fn cw3_controls_cw20() {
    let mut router = mock_app();

    // setup cw3 multisig with 3 accounts
    let cw3_id = router.store_code(contract_cw3_fixed_multisig());

    let addr1 = HumanAddr::from("addr1");
    let addr2 = HumanAddr::from("addr2");
    let addr3 = HumanAddr::from("addr3");
    let cw3_init_msg = InitMsg {
        voters: vec![
            Voter {
                addr: addr1.clone(),
                weight: 1,
            },
            Voter {
                addr: addr2.clone(),
                weight: 1,
            },
            Voter {
                addr: addr3,
                weight: 1,
            },
        ],
        required_weight: 2,
        max_voting_period: Duration::Height(3),
    };

    let multisig_addr = router
        .instantiate_contract(cw3_id, &addr1.clone(), &cw3_init_msg, &[], "Consortium")
        .unwrap();

    // setup cw20 as cw3 multisig admin
    let cw20_id = router.store_code(contract_cw20());

    let cw20_init_msg = cw20_base::msg::InitMsg {
        name: "Consortium Token".parse().unwrap(),
        symbol: "CST".parse().unwrap(),
        decimals: 6,
        initial_balances: vec![],
        mint: Some(MinterResponse {
            minter: multisig_addr.clone(),
            cap: None,
        }),
    };
    let cw20_addr = router
        .instantiate_contract(cw20_id, &multisig_addr, &cw20_init_msg, &[], "Consortium")
        .unwrap();

    // mint some cw20 tokens according to proposal result
    let mint_recipient = HumanAddr::from("recipient");
    let mint_amount = Uint128(1000);
    let cw20_mint_msg = cw20_base::msg::HandleMsg::Mint {
        recipient: mint_recipient.clone(),
        amount: mint_amount,
    };

    let execute_mint_msg = WasmMsg::Execute {
        contract_addr: cw20_addr.clone(),
        msg: to_binary(&cw20_mint_msg).unwrap(),
        send: vec![],
    };
    let propose_msg = HandleMsg::Propose {
        title: "Mint tokens".to_string(),
        description: "Need to mint tokens".to_string(),
        msgs: vec![execute_mint_msg.into()],
        latest: None,
    };
    // propose mint
    router
        .execute_contract(addr1.clone(), multisig_addr.clone(), &propose_msg, &[])
        .unwrap();

    // second votes
    let vote2_msg = HandleMsg::Vote {
        proposal_id: 1,
        vote: Vote::Yes,
    };
    router
        .execute_contract(addr2.clone(), multisig_addr.clone(), &vote2_msg, &[])
        .unwrap();

    // only 1 vote and msg mint fails
    let execute_proposal_msg = HandleMsg::Execute { proposal_id: 1 };
    // execute mint
    router
        .execute_contract(
            addr1.clone(),
            multisig_addr.clone(),
            &execute_proposal_msg,
            &[],
        )
        .unwrap();

    // check the mint is successful
    let cw20_balance_query = QueryMsg::Balance {
        address: mint_recipient,
    };
    let wasm_query = WasmQuery::Smart {
        contract_addr: cw20_addr,
        msg: to_binary(&cw20_balance_query).unwrap(),
    };
    let query_res = router.query(wasm_query.into()).unwrap();
    let balance: BalanceResponse = from_binary(&query_res).unwrap();

    // compare minted amount
    assert_eq!(balance.balance, mint_amount);
}
