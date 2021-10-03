//! This integration test tries to run and call the generated wasm.
//! It depends on a Wasm build being available, which you can create with `cargo wasm`.
//! Then running `cargo integration-test` will validate we can properly call into that generated Wasm.
//!
//! You can easily convert unit tests to integration tests as follows:
//! 1. Copy them over verbatim
//! 2. Then change
//!      let mut deps = mock_dependencies(&[]);
//!    to
//!      let mut deps = mock_instance(WASM, &[]);
//! 3. If you access raw storage, where ever you see something like:
//!      deps.storage.get(CONFIG_KEY).expect("no data stored");
//!    replace it with:
//!      deps.with_storage(|store| {
//!          let data = store.get(CONFIG_KEY).expect("no data stored");
//!          //...
//!      });
//! 4. Anywhere you see query(deps.as_ref(), mock_env(),...) you must replace it with query(&mut deps, ...)

use cosmwasm_std::testing::{mock_env, mock_info};
use cosmwasm_std::{
    from_binary, Api, BankMsg, Coin, CosmosMsg, Decimal, StdError, SubMsg, Uint128,
};
use terra_cosmwasm::create_swap_msg;

use crate::contract::{execute, instantiate, query};
use crate::math::{decimal_multiplication_in_256, decimal_subtraction_in_256};
use crate::state::{store_holder, store_state, Holder, State};
use crate::testing::mock_querier::{
    mock_dependencies, MOCK_HUB_CONTRACT_ADDR, MOCK_TOKEN_CONTRACT_ADDR,
};
use basset::reward::{
    ConfigResponse, ExecuteMsg, HolderResponse, HoldersResponse, InstantiateMsg, QueryMsg,
    StateResponse,
};
use std::str::FromStr;

const DEFAULT_REWARD_DENOM: &str = "uusd";

fn default_init() -> InstantiateMsg {
    InstantiateMsg {
        hub_contract: String::from(MOCK_HUB_CONTRACT_ADDR),
        reward_denom: DEFAULT_REWARD_DENOM.to_string(),
    }
}

#[test]
fn proper_init() {
    let mut deps = mock_dependencies(&[]);
    let init_msg = default_init();

    let info = mock_info("addr0000", &[]);

    let res = instantiate(deps.as_mut(), mock_env(), info, init_msg).unwrap();
    assert_eq!(0, res.messages.len());

    let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let config_response: ConfigResponse = from_binary(&res).unwrap();
    assert_eq!(
        config_response,
        ConfigResponse {
            hub_contract: String::from(MOCK_HUB_CONTRACT_ADDR),
            reward_denom: DEFAULT_REWARD_DENOM.to_string(),
        }
    );

    let res = query(deps.as_ref(), mock_env(), QueryMsg::State {}).unwrap();
    let state_response: StateResponse = from_binary(&res).unwrap();
    assert_eq!(
        state_response,
        StateResponse {
            global_index: Decimal::zero(),
            total_balance: Uint128::new(0u128),
            prev_reward_balance: Uint128::zero()
        }
    );
}

#[test]
pub fn swap_to_reward_denom() {
    let mut deps = mock_dependencies(&[
        Coin {
            denom: "uusd".to_string(),
            amount: Uint128::new(100u128),
        },
        Coin {
            denom: "ukrw".to_string(),
            amount: Uint128::new(1000u128),
        },
        Coin {
            denom: "usdr".to_string(),
            amount: Uint128::new(50u128),
        },
        Coin {
            denom: "mnt".to_string(),
            amount: Uint128::new(50u128),
        },
        Coin {
            denom: "uinr".to_string(),
            amount: Uint128::new(50u128),
        },
    ]);

    let init_msg = default_init();
    let info = mock_info("addr0000", &[]);

    instantiate(deps.as_mut(), mock_env(), info, init_msg).unwrap();

    let info = mock_info(String::from(MOCK_HUB_CONTRACT_ADDR).as_str(), &[]);
    let msg = ExecuteMsg::SwapToRewardDenom {};

    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        res.messages,
        vec![
            SubMsg::new(create_swap_msg(
                Coin {
                    denom: "ukrw".to_string(),
                    amount: Uint128::new(1000u128),
                },
                DEFAULT_REWARD_DENOM.to_string()
            )),
            SubMsg::new(create_swap_msg(
                Coin {
                    denom: "usdr".to_string(),
                    amount: Uint128::new(50u128)
                },
                DEFAULT_REWARD_DENOM.to_string()
            )),
            SubMsg::new(create_swap_msg(
                Coin {
                    denom: "uinr".to_string(),
                    amount: Uint128::new(50u128)
                },
                DEFAULT_REWARD_DENOM.to_string()
            )),
        ]
    );
}

#[test]
fn update_global_index() {
    let mut deps = mock_dependencies(&[Coin {
        denom: "uusd".to_string(),
        amount: Uint128::new(100u128),
    }]);

    let init_msg = default_init();
    let info = mock_info("addr0000", &[]);

    instantiate(deps.as_mut(), mock_env(), info, init_msg).unwrap();

    let msg = ExecuteMsg::UpdateGlobalIndex {};

    // Failed unauthorized try
    let info = mock_info("addr0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg.clone());
    match res {
        Err(_e) => {}
        _ => panic!("DO NOT ENTER HERE"),
    }

    // Failed zero staking balance
    let info = mock_info(MOCK_HUB_CONTRACT_ADDR, &[]);
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone());
    match res {
        Err(StdError::GenericErr { msg, .. }) => assert_eq!(msg, "No asset is bonded by Hub"),
        _ => panic!("DO NOT ENTER HERE"),
    }

    store_state(
        &mut deps.storage,
        &State {
            global_index: Decimal::zero(),
            total_balance: Uint128::from(100u128),
            prev_reward_balance: Uint128::zero(),
        },
    )
    .unwrap();

    // claimed_rewards = 100, total_balance = 100
    // global_index == 1
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let res = query(deps.as_ref(), mock_env(), QueryMsg::State {}).unwrap();
    let state_response: StateResponse = from_binary(&res).unwrap();
    assert_eq!(
        state_response,
        StateResponse {
            global_index: Decimal::one(),
            total_balance: Uint128::from(100u128),
            prev_reward_balance: Uint128::from(100u128)
        }
    );
}

#[test]
fn increase_balance() {
    let mut deps = mock_dependencies(&[Coin {
        denom: "uusd".to_string(),
        amount: Uint128::new(100u128),
    }]);

    let init_msg = default_init();
    let info = mock_info("addr0000", &[]);

    instantiate(deps.as_mut(), mock_env(), info, init_msg).unwrap();

    let msg = ExecuteMsg::IncreaseBalance {
        address: String::from("addr0000"),
        amount: Uint128::from(100u128),
    };

    let info = mock_info("addr0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg.clone());
    match res {
        Err(_e) => {}
        _ => panic!("DO NOT ENTER HERE"),
    };

    let info = mock_info(MOCK_TOKEN_CONTRACT_ADDR, &[]);
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Holder {
            address: String::from("addr0000"),
        },
    )
    .unwrap();
    let holder_response: HolderResponse = from_binary(&res).unwrap();
    assert_eq!(
        holder_response,
        HolderResponse {
            address: String::from("addr0000"),
            balance: Uint128::from(100u128),
            index: Decimal::zero(),
            pending_rewards: Decimal::zero(),
        }
    );

    // claimed_rewards = 100, total_balance = 100
    // global_index == 1
    let info = mock_info(MOCK_HUB_CONTRACT_ADDR, &[]);
    let msg = ExecuteMsg::UpdateGlobalIndex {};
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let info = mock_info(MOCK_TOKEN_CONTRACT_ADDR, &[]);
    let msg = ExecuteMsg::IncreaseBalance {
        address: String::from("addr0000"),
        amount: Uint128::from(100u128),
    };
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Holder {
            address: String::from("addr0000"),
        },
    )
    .unwrap();
    let holder_response: HolderResponse = from_binary(&res).unwrap();
    assert_eq!(
        holder_response,
        HolderResponse {
            address: String::from("addr0000"),
            balance: Uint128::from(200u128),
            index: Decimal::one(),
            pending_rewards: Decimal::from_str("100").unwrap(),
        }
    );
}

#[test]
fn increase_balance_with_decimals() {
    let mut deps = mock_dependencies(&[Coin {
        denom: "uusd".to_string(),
        amount: Uint128::new(100000u128),
    }]);

    let init_msg = default_init();
    let info = mock_info("addr0000", &[]);

    instantiate(deps.as_mut(), mock_env(), info, init_msg).unwrap();

    let msg = ExecuteMsg::IncreaseBalance {
        address: String::from("addr0000"),
        amount: Uint128::from(11u128),
    };

    let info = mock_info("addr0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg.clone());
    match res {
        Err(_e) => {}
        _ => panic!("DO NOT ENTER HERE"),
    };

    let info = mock_info(MOCK_TOKEN_CONTRACT_ADDR, &[]);
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Holder {
            address: String::from("addr0000"),
        },
    )
    .unwrap();
    let holder_response: HolderResponse = from_binary(&res).unwrap();
    assert_eq!(
        holder_response,
        HolderResponse {
            address: String::from("addr0000"),
            balance: Uint128::from(11u128),
            index: Decimal::zero(),
            pending_rewards: Decimal::zero(),
        }
    );

    // claimed_rewards = 100000 , total_balance = 11
    // global_index == 9077.727272727272727272
    let info = mock_info(MOCK_HUB_CONTRACT_ADDR, &[]);
    let msg = ExecuteMsg::UpdateGlobalIndex {};
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let info = mock_info(MOCK_TOKEN_CONTRACT_ADDR, &[]);
    let msg = ExecuteMsg::IncreaseBalance {
        address: String::from("addr0000"),
        amount: Uint128::from(10u128),
    };
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Holder {
            address: String::from("addr0000"),
        },
    )
    .unwrap();
    let holder_response: HolderResponse = from_binary(&res).unwrap();
    let index = decimal_multiplication_in_256(
        Decimal::from_ratio(Uint128::new(100000), Uint128::new(11)),
        Decimal::one(),
    );
    let user_pend_reward = decimal_multiplication_in_256(
        Decimal::from_str("11").unwrap(),
        decimal_subtraction_in_256(holder_response.index, Decimal::zero()),
    );
    assert_eq!(
        holder_response,
        HolderResponse {
            address: String::from("addr0000"),
            balance: Uint128::from(21u128),
            index,
            pending_rewards: user_pend_reward,
        }
    );
}

#[test]
fn decrease_balance() {
    let mut deps = mock_dependencies(&[Coin {
        denom: "uusd".to_string(),
        amount: Uint128::new(100u128),
    }]);

    let init_msg = default_init();
    let info = mock_info("addr0000", &[]);

    instantiate(deps.as_mut(), mock_env(), info, init_msg).unwrap();

    let msg = ExecuteMsg::DecreaseBalance {
        address: String::from("addr0000"),
        amount: Uint128::from(100u128),
    };

    // Failed unautorized
    let info = mock_info("addr0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg.clone());
    match res {
        Err(_e) => {}
        _ => panic!("DO NOT ENTER HERE"),
    };

    // Failed underflow
    let info = mock_info(MOCK_TOKEN_CONTRACT_ADDR, &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg);
    match res {
        Err(StdError::GenericErr { msg, .. }) => {
            assert_eq!(msg, "Decrease amount cannot exceed user balance: 0")
        }
        _ => panic!("DO NOT ENTER HERE"),
    };

    // Increase balance first
    let msg = ExecuteMsg::IncreaseBalance {
        address: String::from("addr0000"),
        amount: Uint128::from(100u128),
    };

    let info = mock_info(MOCK_TOKEN_CONTRACT_ADDR, &[]);
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // claimed_rewards = 100, total_balance = 100
    // global_index == 1
    let info = mock_info(MOCK_HUB_CONTRACT_ADDR, &[]);
    let msg = ExecuteMsg::UpdateGlobalIndex {};
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let info = mock_info(MOCK_TOKEN_CONTRACT_ADDR, &[]);
    let msg = ExecuteMsg::DecreaseBalance {
        address: String::from("addr0000"),
        amount: Uint128::from(100u128),
    };
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Holder {
            address: String::from("addr0000"),
        },
    )
    .unwrap();
    let holder_response: HolderResponse = from_binary(&res).unwrap();
    assert_eq!(
        holder_response,
        HolderResponse {
            address: String::from("addr0000"),
            balance: Uint128::zero(),
            index: Decimal::one(),
            pending_rewards: Decimal::from_str("100").unwrap(),
        }
    );
}

#[test]
fn claim_rewards() {
    let mut deps = mock_dependencies(&[Coin {
        denom: "uusd".to_string(),
        amount: Uint128::new(100u128),
    }]);

    let init_msg = default_init();
    let info = mock_info("addr0000", &[]);

    instantiate(deps.as_mut(), mock_env(), info, init_msg).unwrap();

    let msg = ExecuteMsg::IncreaseBalance {
        address: String::from("addr0000"),
        amount: Uint128::from(100u128),
    };

    let info = mock_info(MOCK_TOKEN_CONTRACT_ADDR, &[]);
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Holder {
            address: String::from("addr0000"),
        },
    )
    .unwrap();
    let holder_response: HolderResponse = from_binary(&res).unwrap();
    assert_eq!(
        holder_response,
        HolderResponse {
            address: String::from("addr0000"),
            balance: Uint128::from(100u128),
            index: Decimal::zero(),
            pending_rewards: Decimal::zero(),
        }
    );

    // claimed_rewards = 100, total_balance = 100
    // global_index == 1
    let info = mock_info(MOCK_HUB_CONTRACT_ADDR, &[]);
    let msg = ExecuteMsg::UpdateGlobalIndex {};
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = ExecuteMsg::ClaimRewards { recipient: None };
    let info = mock_info("addr0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: String::from("addr0000"),
            amount: vec![Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(99u128), // 1% tax
            },]
        }))]
    );

    // Set recipient
    // claimed_rewards = 100, total_balance = 100
    // global_index == 1
    let info = mock_info(MOCK_HUB_CONTRACT_ADDR, &[]);
    let msg = ExecuteMsg::UpdateGlobalIndex {};
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = ExecuteMsg::ClaimRewards {
        recipient: Some(String::from("addr0001")),
    };
    let info = mock_info("addr0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: String::from("addr0001"),
            amount: vec![Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(99u128), // 1% tax
            },]
        }))]
    );
}

#[test]
fn claim_rewards_with_decimals() {
    let mut deps = mock_dependencies(&[Coin {
        denom: "uusd".to_string(),
        amount: Uint128::new(99999u128),
    }]);

    let init_msg = default_init();
    let info = mock_info("addr0000", &[]);

    instantiate(deps.as_mut(), mock_env(), info, init_msg).unwrap();

    let msg = ExecuteMsg::IncreaseBalance {
        address: String::from("addr0000"),
        amount: Uint128::from(11u128),
    };

    let info = mock_info(MOCK_TOKEN_CONTRACT_ADDR, &[]);
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Holder {
            address: String::from("addr0000"),
        },
    )
    .unwrap();
    let holder_response: HolderResponse = from_binary(&res).unwrap();
    assert_eq!(
        holder_response,
        HolderResponse {
            address: String::from("addr0000"),
            balance: Uint128::from(11u128),
            index: Decimal::zero(),
            pending_rewards: Decimal::zero(),
        }
    );

    // claimed_rewards = 1000000, total_balance = 11
    // global_index ==
    let info = mock_info(MOCK_HUB_CONTRACT_ADDR, &[]);

    let msg = ExecuteMsg::UpdateGlobalIndex {};
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = ExecuteMsg::ClaimRewards { recipient: None };
    let info = mock_info("addr0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: String::from("addr0000"),
            amount: vec![Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(99007u128), // 1% tax
            },]
        }))]
    );

    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Holder {
            address: String::from("addr0000"),
        },
    )
    .unwrap();
    let holder_response: HolderResponse = from_binary(&res).unwrap();
    let index = decimal_multiplication_in_256(
        Decimal::from_ratio(Uint128::new(99999), Uint128::new(11)),
        Decimal::one(),
    );
    assert_eq!(
        holder_response,
        HolderResponse {
            address: String::from("addr0000"),
            balance: Uint128::from(11u128),
            index,
            pending_rewards: Decimal::from_str("0.999999999999999991").unwrap(),
        }
    );

    let res = query(deps.as_ref(), mock_env(), QueryMsg::State {}).unwrap();
    let state_response: StateResponse = from_binary(&res).unwrap();
    assert_eq!(
        state_response,
        StateResponse {
            global_index: index,
            total_balance: Uint128::new(11u128),
            prev_reward_balance: Uint128::new(1)
        }
    );
}

#[test]
fn query_holders() {
    let mut deps = mock_dependencies(&[Coin {
        denom: "uusd".to_string(),
        amount: Uint128::new(100u128),
    }]);

    let init_msg = default_init();
    let info = mock_info("addr0000", &[]);

    instantiate(deps.as_mut(), mock_env(), info, init_msg).unwrap();

    let msg = ExecuteMsg::IncreaseBalance {
        address: String::from("addr0000"),
        amount: Uint128::from(100u128),
    };

    let info = mock_info(MOCK_TOKEN_CONTRACT_ADDR, &[]);
    execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let msg = ExecuteMsg::IncreaseBalance {
        address: String::from("addr0001"),
        amount: Uint128::from(200u128),
    };

    execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    let msg = ExecuteMsg::IncreaseBalance {
        address: String::from("addr0002"),
        amount: Uint128::from(300u128),
    };

    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Holders {
            start_after: None,
            limit: None,
        },
    )
    .unwrap();
    let holders_response: HoldersResponse = from_binary(&res).unwrap();
    assert_eq!(
        holders_response,
        HoldersResponse {
            holders: vec![
                HolderResponse {
                    address: String::from("addr0000"),
                    balance: Uint128::from(100u128),
                    index: Decimal::zero(),
                    pending_rewards: Decimal::zero(),
                },
                HolderResponse {
                    address: String::from("addr0001"),
                    balance: Uint128::from(200u128),
                    index: Decimal::zero(),
                    pending_rewards: Decimal::zero(),
                },
                HolderResponse {
                    address: String::from("addr0002"),
                    balance: Uint128::from(300u128),
                    index: Decimal::zero(),
                    pending_rewards: Decimal::zero(),
                },
            ],
        }
    );

    // Set limit
    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Holders {
            start_after: None,
            limit: Some(1),
        },
    )
    .unwrap();
    let holders_response: HoldersResponse = from_binary(&res).unwrap();
    assert_eq!(
        holders_response,
        HoldersResponse {
            holders: vec![HolderResponse {
                address: String::from("addr0000"),
                balance: Uint128::from(100u128),
                index: Decimal::zero(),
                pending_rewards: Decimal::zero(),
            }],
        }
    );

    // Set start_after
    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Holders {
            start_after: Some(String::from("addr0000")),
            limit: None,
        },
    )
    .unwrap();
    let holders_response: HoldersResponse = from_binary(&res).unwrap();
    assert_eq!(
        holders_response,
        HoldersResponse {
            holders: vec![
                HolderResponse {
                    address: String::from("addr0001"),
                    balance: Uint128::from(200u128),
                    index: Decimal::zero(),
                    pending_rewards: Decimal::zero(),
                },
                HolderResponse {
                    address: String::from("addr0002"),
                    balance: Uint128::from(300u128),
                    index: Decimal::zero(),
                    pending_rewards: Decimal::zero(),
                }
            ],
        }
    );

    // Set start_after and limit
    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Holders {
            start_after: Some(String::from("addr0000")),
            limit: Some(1),
        },
    )
    .unwrap();
    let holders_response: HoldersResponse = from_binary(&res).unwrap();
    assert_eq!(
        holders_response,
        HoldersResponse {
            holders: vec![HolderResponse {
                address: String::from("addr0001"),
                balance: Uint128::from(200u128),
                index: Decimal::zero(),
                pending_rewards: Decimal::zero(),
            }],
        }
    );
}

#[test]
fn proper_prev_balance() {
    let mut deps = mock_dependencies(&[Coin {
        denom: "uusd".to_string(),
        amount: Uint128::new(100u128),
    }]);

    let init_msg = default_init();
    let info = mock_info("addr0000", &[]);

    instantiate(deps.as_mut(), mock_env(), info, init_msg).unwrap();

    let amount1 = Uint128::from(8899999999988889u128);
    let amount2 = Uint128::from(14487875351811111u128);
    let amount3 = Uint128::from(1100000000000000u128);

    let rewards = Uint128::new(677101666827000000u128);

    let all_balance = amount1 + amount2 + amount3;

    let global_index = Decimal::from_ratio(rewards, all_balance);
    store_state(
        &mut deps.storage,
        &State {
            global_index,
            total_balance: all_balance,
            prev_reward_balance: rewards,
        },
    )
    .unwrap();

    let holder = Holder {
        balance: amount1,
        index: Decimal::from_str("0").unwrap(),
        pending_rewards: Decimal::from_str("0").unwrap(),
    };
    store_holder(
        &mut deps.storage,
        &deps
            .api
            .addr_canonicalize(&String::from("addr0000"))
            .unwrap(),
        &holder,
    )
    .unwrap();

    let holder = Holder {
        balance: amount2,
        index: Decimal::from_str("0").unwrap(),
        pending_rewards: Decimal::from_str("0").unwrap(),
    };
    store_holder(
        &mut deps.storage,
        &deps
            .api
            .addr_canonicalize(&String::from("addr0001"))
            .unwrap(),
        &holder,
    )
    .unwrap();

    let holder = Holder {
        balance: amount3,
        index: Decimal::from_str("0").unwrap(),
        pending_rewards: Decimal::from_str("0").unwrap(),
    };
    store_holder(
        &mut deps.storage,
        &deps
            .api
            .addr_canonicalize(&String::from("addr0002"))
            .unwrap(),
        &holder,
    )
    .unwrap();

    let msg = ExecuteMsg::ClaimRewards { recipient: None };
    let info = mock_info("addr0000", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = ExecuteMsg::ClaimRewards { recipient: None };
    let info = mock_info("addr0001", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = ExecuteMsg::ClaimRewards { recipient: None };
    let info = mock_info("addr0002", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let res = query(deps.as_ref(), mock_env(), QueryMsg::State {}).unwrap();
    let state_response: StateResponse = from_binary(&res).unwrap();
    assert_eq!(
        state_response,
        StateResponse {
            global_index,
            total_balance: all_balance,
            prev_reward_balance: Uint128::new(1)
        }
    );

    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Holder {
            address: String::from("addr0000"),
        },
    )
    .unwrap();
    let holder_response: HolderResponse = from_binary(&res).unwrap();
    assert_eq!(
        holder_response,
        HolderResponse {
            address: String::from("addr0000"),
            balance: amount1,
            index: global_index,
            pending_rewards: Decimal::from_str("0.212799238975421283").unwrap(),
        }
    );

    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Holder {
            address: String::from("addr0001"),
        },
    )
    .unwrap();
    let holder_response: HolderResponse = from_binary(&res).unwrap();
    assert_eq!(
        holder_response,
        HolderResponse {
            address: String::from("addr0001"),
            balance: amount2,
            index: global_index,
            pending_rewards: Decimal::from_str("0.078595712259178717").unwrap(),
        }
    );

    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Holder {
            address: String::from("addr0002"),
        },
    )
    .unwrap();
    let holder_response: HolderResponse = from_binary(&res).unwrap();
    assert_eq!(
        holder_response,
        HolderResponse {
            address: String::from("addr0002"),
            balance: amount3,
            index: global_index,
            pending_rewards: Decimal::from_str("0.701700000000000000").unwrap(),
        }
    );
}
