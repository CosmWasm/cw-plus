use cosmwasm_std::Empty;
use cosmwasm_std::{coins, to_binary, BankMsg, Response, WasmMsg};
use cosmwasm_vm::testing::{
    execute, instantiate, mock_env, mock_info, mock_instance_with_gas_limit,
};
use cw1_whitelist::msg::{ExecuteMsg, InstantiateMsg};

static WASM: &[u8] = include_bytes!("../../../artifacts/cw1_whitelist.wasm");

#[test]
fn execute_bench() {
    let mut deps = mock_instance_with_gas_limit(WASM, 100_000_000_000_000);

    let alice = "alice";
    let bob = "bob";
    let carl = "carl";

    // instantiate the contract
    let instantiate_msg = InstantiateMsg {
        admins: vec![alice.to_string(), carl.to_string()],
        mutable: false,
    };
    let info = mock_info(&bob, &[]);

    let init_gas = deps.get_gas_left();

    let _: Response = instantiate(&mut deps, mock_env(), info, instantiate_msg).unwrap();

    let gas_after_instantiation = deps.get_gas_left();

    let freeze: ExecuteMsg<Empty> = ExecuteMsg::Freeze {};
    let msgs = vec![
        BankMsg::Send {
            to_address: bob.to_string(),
            amount: coins(10000, "DAI"),
        }
        .into(),
        WasmMsg::Execute {
            contract_addr: "some contract".into(),
            msg: to_binary(&freeze).unwrap(),
            funds: vec![],
        }
        .into(),
    ];

    // make some nice message
    let execute_msg = ExecuteMsg::<Empty>::Execute { msgs: msgs.clone() };

    // but carl can
    let info = mock_info(&carl, &[]);
    let _: Response = execute(&mut deps, mock_env(), info, execute_msg).unwrap();

    let gas_after_execution = deps.get_gas_left();

    println!(
        "Instantiation gas usage: {}",
        init_gas - gas_after_instantiation
    );
    println!(
        "Execution gas usage: {}",
        gas_after_instantiation - gas_after_execution
    );
}
