#![cfg(test)]

use crate::contract::{execute, instantiate, query};
use crate::msg::{ExecuteMsg, InstantiateMsg, Voter};
use cosmwasm_std::{to_binary, Empty, Uint128, WasmMsg};
use cw0::Duration;
use cw20::{BalanceResponse, MinterResponse};
use cw20_base::msg::QueryMsg;
use cw3::Vote;
use cw_multi_test::{AppBuilder, Contract, ContractWrapper, Executor};

pub fn contract_cw3_fixed_multisig() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(execute, instantiate, query);
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
// cw3 multisig account can control cw20 admin actions
fn cw3_controls_cw20() {
    let mut app = AppBuilder::new().build();

    // setup cw3 multisig with 3 accounts
    let cw3_id = app.store_code(contract_cw3_fixed_multisig());

    let addr1 = app.api().addr_validate("addr1").unwrap();
    let addr2 = app.api().addr_validate("addr2").unwrap();
    let addr3 = app.api().addr_validate("addr3").unwrap();
    let cw3_instantiate_msg = InstantiateMsg {
        voters: vec![
            Voter {
                addr: addr1.to_string(),
                weight: 1,
            },
            Voter {
                addr: addr2.to_string(),
                weight: 1,
            },
            Voter {
                addr: addr3.to_string(),
                weight: 1,
            },
        ],
        required_weight: 2,
        max_voting_period: Duration::Height(3),
    };

    let multisig_addr = app
        .instantiate_contract(
            cw3_id,
            addr1.clone(),
            &cw3_instantiate_msg,
            &[],
            "Consortium",
            None,
        )
        .unwrap();

    // setup cw20 as cw3 multisig admin
    let cw20_id = app.store_code(contract_cw20());

    let cw20_instantiate_msg = cw20_base::msg::InstantiateMsg {
        name: "Consortium Token".parse().unwrap(),
        symbol: "CST".parse().unwrap(),
        decimals: 6,
        initial_balances: vec![],
        mint: Some(MinterResponse {
            minter: multisig_addr.to_string(),
            cap: None,
        }),
        marketing: None,
    };
    let cw20_addr = app
        .instantiate_contract(
            cw20_id,
            multisig_addr.clone(),
            &cw20_instantiate_msg,
            &[],
            "Consortium",
            None,
        )
        .unwrap();

    // mint some cw20 tokens according to proposal result
    let mint_recipient = app.api().addr_validate("recipient").unwrap();
    let mint_amount = Uint128::new(1000);
    let cw20_mint_msg = cw20_base::msg::ExecuteMsg::Mint {
        recipient: mint_recipient.to_string(),
        amount: mint_amount,
    };

    let execute_mint_msg = WasmMsg::Execute {
        contract_addr: cw20_addr.to_string(),
        msg: to_binary(&cw20_mint_msg).unwrap(),
        funds: vec![],
    };
    let propose_msg = ExecuteMsg::Propose {
        title: "Mint tokens".to_string(),
        description: "Need to mint tokens".to_string(),
        msgs: vec![execute_mint_msg.into()],
        latest: None,
    };
    // propose mint
    app.execute_contract(addr1.clone(), multisig_addr.clone(), &propose_msg, &[])
        .unwrap();

    // second votes
    let vote2_msg = ExecuteMsg::Vote {
        proposal_id: 1,
        vote: Vote::Yes,
    };
    app.execute_contract(addr2, multisig_addr.clone(), &vote2_msg, &[])
        .unwrap();

    // only 1 vote and msg mint fails
    let execute_proposal_msg = ExecuteMsg::Execute { proposal_id: 1 };
    // execute mint
    app.execute_contract(addr1, multisig_addr, &execute_proposal_msg, &[])
        .unwrap();

    // check the mint is successful
    let cw20_balance_query = QueryMsg::Balance {
        address: mint_recipient.to_string(),
    };
    let balance: BalanceResponse = app
        .wrap()
        .query_wasm_smart(&cw20_addr, &cw20_balance_query)
        .unwrap();

    // compare minted amount
    assert_eq!(balance.balance, mint_amount);
}
