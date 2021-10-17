//! This integration test tries to run and call the generated wasm.
//! It depends on a Wasm build being available, which you can create with `cargo wasm`.
//! Then running `cargo integration-test` will validate we can properly call into that generated Wasm.
//!
//! You can easily convert unit tests to integration tests as follows:
//! 1. Copy them over verbatim
//! 2. Then change
//!      let mut deps = mock_dependencies(20, &[]);
//!    to
//!      let mut deps = mock_instance(WASM, &[]);
//! 3. If you access raw storage, where ever you see something like:
//!      deps.storage.get(CONFIG_KEY).expect("no data stored");
//!    replace it with:
//!      deps.with_storage(|store| {
//!          let data = store.get(CONFIG_KEY).expect("no data stored");
//!          //...
//!      });
//! 4. Anywhere you see query(&deps, ...) you must replace it with query(&mut deps, ...)

#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{
        from_binary, to_binary, Addr, Api, BankMsg, Coin, CosmosMsg, Decimal, Empty, MessageInfo,
        StdError, SubMsg, Uint128, WasmMsg,
    };

    use crate::contract::{calculate_decimal_rewards, execute, get_decimals, instantiate, query};
    use crate::msg::{
        ExecuteMsg, HolderResponse, HoldersResponse, InstantiateMsg, QueryMsg, ReceiveMsg,
        RewardIndexResponse, StateResponse,
    };

    use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};

    use crate::state::{Holder, State, HOLDERS, STATE};
    use std::borrow::{Borrow, BorrowMut};
    use std::ops::{Mul, Sub};
    use std::str::FromStr;

    const DEFAULT_REWARD_DENOM: &str = "uusd";
    const MOCK_CW20_CONTRACT_ADDR: &str = "cw20";
    const MOCK_TOKEN_CONTRACT_ADDR: &str = "token";
    fn default_init() -> InstantiateMsg {
        InstantiateMsg {
            cw20_token_addr: Addr::unchecked(MOCK_CW20_CONTRACT_ADDR),
            unbonding_period: 1000,
            native_denom: DEFAULT_REWARD_DENOM.to_string(),
        }
    }

    fn receive_stake_msg(sender: &str, amount: u128) -> ExecuteMsg {
        let bond_msg = ReceiveMsg::BondStake {};
        let cw20_receive_msg = Cw20ReceiveMsg {
            sender: sender.to_string(),
            amount: Uint128::new(amount),
            msg: to_binary(&bond_msg).unwrap(),
        };
        ExecuteMsg::Receive(cw20_receive_msg)
    }

    #[test]
    fn proper_init() {
        let mut deps = mock_dependencies(&[]);
        let init_msg = default_init();
        let env = mock_env();
        let info = MessageInfo {
            sender: Addr::unchecked("ok"),
            funds: vec![],
        };
        let res = instantiate(deps.as_mut(), env.clone(), info, init_msg).unwrap();
        assert_eq!(0, res.messages.len());

        let res = query(deps.as_ref(), env.clone(), QueryMsg::State {}).unwrap();
        let config_response: StateResponse = from_binary(&res).unwrap();
        assert_eq!(
            config_response,
            StateResponse {
                cw20_token_addr: MOCK_CW20_CONTRACT_ADDR.to_string(),
                unbonding_period: 1000,
                native_denom: DEFAULT_REWARD_DENOM.to_string(),
                total_balance: Default::default(),
                global_index: Decimal::zero(),
                prev_reward_balance: Default::default()
            }
        );

        let res = query(
            deps.as_ref(),
            env,
            QueryMsg::RewardIndex {
                cw20_addr: MOCK_CW20_CONTRACT_ADDR.to_string(),
            },
        )
        .unwrap();
        let state_response: RewardIndexResponse = from_binary(&res).unwrap();
        assert_eq!(
            state_response,
            RewardIndexResponse {
                global_index: Decimal::zero(),
                total_balance: Uint128::new(0u128),
                prev_reward_balance: Uint128::zero()
            }
        );
    }

    #[test]
    fn update_global_index() {
        let mut deps = mock_dependencies(&[Coin {
            denom: "coin".to_string(),
            amount: Uint128::new(100u128),
        }]);

        let init_msg = default_init();
        let env = mock_env();
        let info = mock_info("sender", &[]);
        instantiate(deps.as_mut(), env.clone(), info.clone(), init_msg);

        let msg = ExecuteMsg::UpdateRewardIndex {};

        // Failed zero staking balance
        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());
        match res {
            Err(StdError::GenericErr { msg, .. }) => assert_eq!(msg, "No asset is bonded by Hub"),
            _ => panic!("DO NOT ENTER HERE"),
        }
        STATE
            .save(
                deps.as_mut().storage,
                &State {
                    cw20_token_addr: MOCK_CW20_CONTRACT_ADDR,
                    unbonding_period: 0,
                    global_index: Decimal::zero(),
                    total_balance: Uint128::from(100u128),
                    prev_reward_balance: Uint128::zero(),
                },
            )
            .unwrap();

        // claimed_rewards = 100, total_balance = 100
        // global_index == 1
        //handle(&mut deps, env, msg).unwrap();
        execute(deps.as_mut(), env.clone(), info, msg).unwrap();
        let res = query(
            deps.as_ref(),
            env,
            QueryMsg::RewardIndex {
                cw20_addr: MOCK_CW20_CONTRACT_ADDR.into_string(),
            },
        )
        .unwrap();
        let state_response: RewardIndexResponse = from_binary(&res).unwrap();
        assert_eq!(
            state_response,
            RewardIndexResponse {
                global_index: Decimal::one(),
                total_balance: Uint128::from(100u128),
                prev_reward_balance: Uint128::from(100u128)
            }
        );
    }

    #[test]
    fn increase_balance() {
        let mut deps = mock_dependencies(&[Coin {
            denom: DEFAULT_REWARD_DENOM.to_string(),
            amount: Uint128::new(100u128),
        }]);

        let init_msg = default_init();
        let env = mock_env();
        let info = mock_info("addr0000", &[]);
        instantiate(deps.as_mut(), env.clone(), info.clone(), init_msg);

        let info = mock_info(MOCK_CW20_CONTRACT_ADDR, &[]);
        let receive_msg = receive_stake_msg("addr0000", 100);
        execute(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            receive_msg.clone(),
        );

        let res = query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::Holder {
                address: "addr0000".to_string(),
            },
        )
        .unwrap();

        let holder_response: HolderResponse = from_binary(&res).unwrap();
        assert_eq!(
            holder_response,
            HolderResponse {
                address: "addr0000".to_string(),
                balance: Uint128::from(100u128),
                index: Decimal::zero(),
                pending_rewards: Decimal::zero(),
            }
        );

        // claimed_rewards = 100, total_balance = 100
        // global_index == 1
        let info = mock_info("addr0000", &[]);
        let msg = ExecuteMsg::UpdateRewardIndex {
            cw20_addr: MOCK_CW20_CONTRACT_ADDR.into_string(),
        };
        execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let info = mock_info(MOCK_CW20_CONTRACT_ADDR, &[]);
        execute(deps.as_mut(), env.clone(), info.clone(), receive_msg).unwrap();
        let res = query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::Holder {
                address: "addr0000".to_string(),
            },
        )
        .unwrap();

        let holder_response: HolderResponse = from_binary(&res).unwrap();
        assert_eq!(
            holder_response,
            HolderResponse {
                address: "addr0000".to_string(),
                balance: Uint128::from(200u128),
                index: Decimal::one(),
                pending_rewards: Decimal::from_str("100").unwrap(),
            }
        );
    }

    #[test]
    fn increase_balance_with_decimals() {
        let mut deps = mock_dependencies(&[Coin {
            denom: DEFAULT_REWARD_DENOM.to_string(),
            amount: Uint128::new(100000u128),
        }]);

        let init_msg = default_init();
        let env = mock_env();
        let info = mock_info("addr0000", &[]);
        instantiate(deps.as_mut(), env.clone(), info.clone(), init_msg);

        let info = mock_info(MOCK_CW20_CONTRACT_ADDR, &[]);
        let receive_msg = receive_stake_msg("addr0000", 11);
        execute(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            receive_msg.clone(),
        )
        .unwrap();
        let res = query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::Holder {
                address: "addr0000".to_string(),
            },
        )
        .unwrap();

        let holder_response: HolderResponse = from_binary(&res).unwrap();
        assert_eq!(
            holder_response,
            HolderResponse {
                address: "addr0000".to_string(),
                balance: Uint128::from(11u128),
                index: Decimal::zero(),
                pending_rewards: Decimal::zero(),
            }
        );

        // claimed_rewards = 100000 , total_balance = 11
        // global_index == 9077.727272727272727272
        let info = mock_info("addr0000", &[]);
        let msg = ExecuteMsg::UpdateRewardIndex {
            cw20_addr: MOCK_CW20_CONTRACT_ADDR.into_string(),
        };
        execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let info = mock_info(MOCK_CW20_CONTRACT_ADDR, &[]);
        let receive_msg = receive_stake_msg("addr0000", 10);
        execute(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            receive_msg.clone(),
        )
        .unwrap();

        let res = query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::Holder {
                address: "addr0000".to_string(),
            },
        )
        .unwrap();

        let holder_response: HolderResponse = from_binary(&res).unwrap();

        let value1 = Decimal::from_ratio(Uint128::new(100000), Uint128::new(11));
        let index = value1.mul(Decimal::one());
        let pend_value1 = holder_response.index.sub(Decimal::zero());
        let user_pend_reward = Decimal::from_str("11").unwrap().mul(pend_value1);
        assert_eq!(
            holder_response,
            HolderResponse {
                address: "addr0000".to_string(),
                balance: Uint128::from(21u128),
                index,
                pending_rewards: user_pend_reward,
            }
        );
    }

    #[test]
    fn unbond_stake() {
        let mut deps = mock_dependencies(&[Coin {
            denom: DEFAULT_REWARD_DENOM.to_string(),
            amount: Uint128::new(100u128),
        }]);

        let init_msg = default_init();
        let env = mock_env();
        let info = mock_info("addr0000", &[]);
        instantiate(deps.as_mut(), env.clone(), info, init_msg);

        let msg = ExecuteMsg::UnbondStake {
            amount: Uint128::from(100u128),
        };

        // Failed underflow
        let info = mock_info("addr0000", &[]);
        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());
        match res {
            Err(StdError::GenericErr { msg, .. }) => {
                assert_eq!(msg, "Decrease amount cannot exceed user balance: 0")
            }
            _ => panic!("DO NOT ENTER HERE"),
        };

        let info = mock_info(MOCK_CW20_CONTRACT_ADDR, &[]);
        let receive_msg = receive_stake_msg("addr0000", 100);
        execute(deps.as_mut(), env.clone(), info, receive_msg).unwrap();

        // claimed_rewards = 100, total_balance = 100
        // global_index == 1
        let info = mock_info("addr0000", &[]);
        let msg = ExecuteMsg::UpdateRewardIndex {
            cw20_addr: MOCK_CW20_CONTRACT_ADDR.into_string(),
        };
        execute(deps.as_mut(), env.clone(), info, msg).unwrap();

        let info = mock_info("addr0000", &[]);
        let msg = ExecuteMsg::UnbondStake {
            amount: Uint128::from(100u128),
        };
        execute(deps.as_mut(), env.clone(), info, msg).unwrap();

        let res = query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::Holder {
                address: "addr0000".to_string(),
            },
        )
        .unwrap();

        let holder_response: HolderResponse = from_binary(&res).unwrap();
        assert_eq!(
            holder_response,
            HolderResponse {
                address: "addr0000".to_string(),
                balance: Uint128::zero(),
                index: Decimal::one(),
                pending_rewards: Decimal::from_str("100").unwrap(),
            }
        );
    }

    #[test]
    fn claim_rewards() {
        let mut deps = mock_dependencies(&[Coin {
            denom: DEFAULT_REWARD_DENOM.to_string(),
            amount: Uint128::new(100u128),
        }]);

        let init_msg = default_init();
        let env = mock_env();
        let info = mock_info(MOCK_TOKEN_CONTRACT_ADDR, &[]);
        instantiate(deps.as_mut(), env.clone(), info, init_msg);

        let info = mock_info(MOCK_CW20_CONTRACT_ADDR, &[]);
        let receive_msg = receive_stake_msg("addr0000", 100);
        execute(deps.as_mut(), env.clone(), info, receive_msg).unwrap();
        let res = query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::Holder {
                address: "addr0000".to_string(),
            },
        )
        .unwrap();

        let holder_response: HolderResponse = from_binary(&res).unwrap();
        assert_eq!(
            holder_response,
            HolderResponse {
                address: "addr0000".to_string(),
                balance: Uint128::from(100u128),
                index: Decimal::zero(),
                pending_rewards: Decimal::zero(),
            }
        );

        // claimed_rewards = 100, total_balance = 100
        // global_index == 1
        let info = mock_info("sender", &[]);
        let msg = ExecuteMsg::UpdateRewardIndex {
            cw20_addr: MOCK_CW20_CONTRACT_ADDR.into_string(),
        };
        execute(deps.as_mut(), env.clone(), info, msg).unwrap();

        let msg = ExecuteMsg::ClaimRewards { recipient: None };
        let info = mock_info("addr0000", &[]);
        let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

        assert_eq!(
            res.messages,
            vec![SubMsg::new(BankMsg::Send {
                to_address: "addr0000".to_string(),
                amount: vec![Coin {
                    denom: "uusd".to_string(),
                    amount: Uint128::from(99u128), // 1% tax
                },]
            })]
        );

        // Set recipient
        // claimed_rewards = 100, total_balance = 100
        // global_index == 1
        let info = mock_info("sender", &[]);
        let msg = ExecuteMsg::UpdateRewardIndex {
            cw20_addr: MOCK_CW20_CONTRACT_ADDR.into_string(),
        };
        execute(deps.as_mut(), env.clone(), info, msg).unwrap();

        let msg = ExecuteMsg::ClaimRewards {
            recipient: Some(Addr::unchecked("addr0001").to_string()),
        };
        let info = mock_info("addr0000", &[]);
        let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
        assert_eq!(
            res.messages,
            vec![SubMsg::new(BankMsg::Send {
                to_address: "addr0001".to_string(),
                amount: vec![Coin {
                    denom: "uusd".to_string(),
                    amount: Uint128::from(99u128), // 1% tax
                },]
            })]
        );
    }

    #[test]
    fn withdraw_stake() {
        let mut deps = mock_dependencies(&[Coin {
            denom: DEFAULT_REWARD_DENOM.to_string(),
            amount: Uint128::new(100u128),
        }]);

        let init_msg = default_init();
        let mut env = mock_env();
        let info = mock_info(MOCK_TOKEN_CONTRACT_ADDR, &[]);
        instantiate(deps.as_mut(), env.clone(), info, init_msg);

        let info = mock_info(MOCK_CW20_CONTRACT_ADDR, &[]);
        let receive_msg = receive_stake_msg("addr0000", 100);
        execute(deps.as_mut(), env.clone(), info, receive_msg.clone()).unwrap();

        let res = query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::Holder {
                address: "addr0000".to_string(),
            },
        )
        .unwrap();

        let holder_response: HolderResponse = from_binary(&res).unwrap();
        assert_eq!(
            holder_response,
            HolderResponse {
                address: "addr0000".to_string(),
                balance: Uint128::from(100u128),
                index: Decimal::zero(),
                pending_rewards: Decimal::zero(),
            }
        );

        // claimed_rewards = 100, total_balance = 100
        // global_index == 1
        let info = mock_info("sender", &[]);
        let msg = ExecuteMsg::UpdateRewardIndex {
            cw20_addr: MOCK_CW20_CONTRACT_ADDR.into_string(),
        };
        execute(deps.as_mut(), env.clone(), info, msg).unwrap();

        let msg = ExecuteMsg::ClaimRewards { recipient: None };
        let info = mock_info("addr0000", &[]);
        let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
        assert_eq!(
            res.messages,
            vec![SubMsg::new(BankMsg::Send {
                to_address: "addr0000".to_string(),
                amount: vec![Coin {
                    denom: "uusd".to_string(),
                    amount: Uint128::from(99u128), // 1% tax
                }]
            })]
        );

        // withdraw stake
        let msg = ExecuteMsg::UnbondStake {
            amount: Uint128::from(100u128),
        };
        let info = mock_info("addr0000", &[]);
        env.block.height = 5;
        execute(deps.as_mut(), env.clone(), info, msg).unwrap();

        // withdraw before unbonding fails
        let msg = ExecuteMsg::WithdrawStake { cap: None };
        let info = mock_info("addr0000", &[]);
        env.block.height = 10;
        let res = execute(deps.as_mut(), env.clone(), info, msg);

        match res {
            Err(StdError::GenericErr { msg, .. }) => {
                assert_eq!(msg, "Wait for the unbonding period")
            }
            _ => panic!("Unexpected error"),
        }

        // withdraw works after unbonding period
        let msg = ExecuteMsg::WithdrawStake { cap: None };
        let info = mock_info("addr0000", &[]);
        env.block.height = 10000;
        let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

        let cw20_transfer_msg = Cw20ExecuteMsg::Transfer {
            recipient: "addr0000".to_string(),
            amount: Uint128::from(100u128),
        };
        assert_eq!(
            res.messages,
            vec![SubMsg::new(WasmMsg::Execute {
                contract_addr: MOCK_CW20_CONTRACT_ADDR.to_string(),
                msg: to_binary(&cw20_transfer_msg).unwrap(),
                funds: vec![]
            })]
        );
    }

    #[test]
    fn withdraw_stake_cap() {
        let mut deps = mock_dependencies(&[Coin {
            denom: DEFAULT_REWARD_DENOM.to_string(),
            amount: Uint128::new(100u128),
        }]);

        let init_msg = default_init();
        let mut env = mock_env();
        let info = mock_info("addr0000", &[]);
        instantiate(deps.as_mut(), env.clone(), info, init_msg);

        let info = mock_info(MOCK_CW20_CONTRACT_ADDR, &[]);
        let receive_msg = receive_stake_msg("addr0000", 100);
        execute(deps.as_mut(), env.clone(), info, receive_msg.clone()).unwrap();

        let res = query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::Holder {
                address: "addr0000".to_string(),
            },
        )
        .unwrap();

        let holder_response: HolderResponse = from_binary(&res).unwrap();
        assert_eq!(
            holder_response,
            HolderResponse {
                address: "addr0000".to_string(),
                balance: Uint128::from(100u128),
                index: Decimal::zero(),
                pending_rewards: Decimal::zero(),
            }
        );

        // claimed_rewards = 100, total_balance = 100
        // global_index == 1
        let info = mock_info("addr0000", &[]);
        let msg = ExecuteMsg::UpdateRewardIndex {
            cw20_addr: MOCK_CW20_CONTRACT_ADDR.into_string(),
        };
        execute(deps.as_mut(), env.clone(), info, msg).unwrap();

        let msg = ExecuteMsg::ClaimRewards { recipient: None };
        let info = mock_info("addr0000", &[]);
        let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
        assert_eq!(
            res.messages,
            vec![SubMsg::new(BankMsg::Send {
                to_address: "addr0000".to_string(),
                amount: vec![Coin {
                    denom: "uusd".to_string(),
                    amount: Uint128::from(99u128), // 1% tax
                }]
            })]
        );

        // withdraw stake
        let msg = ExecuteMsg::UnbondStake {
            amount: Uint128::from(100u128),
        };
        let info = mock_info("addr0000", &[]);
        env.block.height = 5;
        execute(deps.as_mut(), env.clone(), info, msg).unwrap();

        // cap is less then release, wait for more to unbond
        let msg = ExecuteMsg::WithdrawStake {
            cap: Some(Uint128::from(50u128)),
        };
        let info = mock_info("addr0000", &[]);
        env.block.height = 100000;
        let res = execute(deps.as_mut(), env.clone(), info, msg);
        match res {
            Err(StdError::GenericErr { msg, .. }) => {
                assert_eq!(msg, "Wait for the unbonding period")
            }

            _ => panic!("Unexpected error"),
        }

        let msg = ExecuteMsg::WithdrawStake {
            cap: Some(Uint128::from(150u128)),
        };
        let info = mock_info("addr0000", &[]);
        env.block.height = 100000;
        let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

        let cw20_transfer_msg = Cw20ExecuteMsg::Transfer {
            recipient: "addr0000".to_string(),
            amount: Uint128::from(100u128),
        };
        assert_eq!(
            res.messages,
            vec![SubMsg::new(WasmMsg::Execute {
                contract_addr: MOCK_CW20_CONTRACT_ADDR.to_string(),
                msg: to_binary(&cw20_transfer_msg).unwrap(),
                funds: vec![]
            })]
        );
    }

    #[test]
    fn claim_rewards_with_decimals() {
        let mut deps = mock_dependencies(&[Coin {
            denom: DEFAULT_REWARD_DENOM.to_string(),
            amount: Uint128::new(99999u128),
        }]);
        let init_msg = default_init();
        let env = mock_env();
        let info = mock_info("addr0000", &[]);
        instantiate(deps.as_mut(), env.clone(), info, init_msg);

        let info = mock_info(MOCK_CW20_CONTRACT_ADDR, &[]);
        let receive_msg = receive_stake_msg("addr0000", 11);
        execute(deps.as_mut(), env.clone(), info, receive_msg.clone()).unwrap();

        let res = query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::Holder {
                address: "addr0000".to_string(),
            },
        )
        .unwrap();

        let holder_response: HolderResponse = from_binary(&res).unwrap();
        assert_eq!(
            holder_response,
            HolderResponse {
                address: "addr0000".to_string(),
                balance: Uint128::from(11u128),
                index: Decimal::zero(),
                pending_rewards: Decimal::zero(),
            }
        );

        // claimed_rewards = 1000000, total_balance = 11
        // global_index ==
        let info = mock_info("sender", &[]);
        let msg = ExecuteMsg::UpdateRewardIndex {};
        execute(deps.as_mut(), env.clone(), info, msg.clone()).unwrap();

        let msg = ExecuteMsg::ClaimRewards { recipient: None };
        let info = mock_info("addr0000", &[]);
        let res = execute(deps.as_mut(), env.clone(), info, msg.clone()).unwrap();
        assert_eq!(
            res.messages,
            vec![SubMsg::new(BankMsg::Send {
                to_address: "addr0000".to_string(),
                amount: vec![Coin {
                    denom: "uusd".to_string(),
                    amount: Uint128::from(99007u128), // 1% tax
                },]
            })]
        );

        let res = query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::Holder {
                address: "addr0000".to_string(),
            },
        )
        .unwrap();
        let holder_response: HolderResponse = from_binary(&res).unwrap();
        let value1 = Decimal::from_ratio(Uint128::new(99999), Uint128::new(11));
        let index = Decimal::one().mul(value1);
        assert_eq!(
            holder_response,
            HolderResponse {
                address: "addr0000".to_string(),
                balance: Uint128::from(11u128),
                index,
                pending_rewards: Decimal::from_str("0.999999999999999991").unwrap(),
            }
        );

        let res = query(
            deps.as_ref(),
            env,
            QueryMsg::RewardIndex {
                cw20_addr: MOCK_CW20_CONTRACT_ADDR.into_string(),
            },
        )
        .unwrap();
        let state_response: RewardIndexResponse = from_binary(&res).unwrap();
        assert_eq!(
            state_response,
            RewardIndexResponse {
                global_index: index,
                total_balance: Uint128::new(11u128),
                prev_reward_balance: Uint128::new(1)
            }
        );
    }

    #[test]
    fn query_holders() {
        let mut deps = mock_dependencies(&[Coin {
            denom: DEFAULT_REWARD_DENOM.to_string(),
            amount: Uint128::new(100u128),
        }]);

        let init_msg = default_init();
        let env = mock_env();
        let info = mock_info("addr0000", &[]);
        instantiate(deps.as_mut(), env.clone(), info, init_msg);

        let info = mock_info(MOCK_CW20_CONTRACT_ADDR, &[]);
        let receive_msg = receive_stake_msg(Addr::unchecked("addr0000").as_str(), 100);
        execute(deps.as_mut(), env.clone(), info, receive_msg.clone()).unwrap();

        let info = mock_info(MOCK_CW20_CONTRACT_ADDR, &[]);
        let receive_msg = receive_stake_msg(Addr::unchecked("addr0001").as_str(), 200);
        execute(deps.as_mut(), env.clone(), info, receive_msg.clone()).unwrap();

        let info = mock_info(MOCK_CW20_CONTRACT_ADDR, &[]);
        let receive_msg = receive_stake_msg(Addr::unchecked("addr0002").as_str(), 300);
        execute(deps.as_mut(), env.clone(), info, receive_msg.clone()).unwrap();

        let res = query(
            deps.as_ref(),
            env.clone(),
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
                    }
                ],
            }
        );

        // Set limit
        let res = query(
            deps.as_ref(),
            env.clone(),
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
            env.clone(),
            QueryMsg::Holders {
                start_after: Some(String::from("addr0002")),
                limit: None,
            },
        )
        .unwrap();
        let holders_response: HoldersResponse = from_binary(&res).unwrap();
        assert_eq!(holders_response, HoldersResponse { holders: vec![] });

        // Set start_after and limit
        let res = query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::Holders {
                start_after: Some("addr0000".to_string()),
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
            denom: DEFAULT_REWARD_DENOM.to_string(),
            amount: Uint128::new(100u128),
        }]);

        let init_msg = default_init();
        let env = mock_env();
        let info = mock_info("addr0000", &[]);
        instantiate(deps.as_mut(), env.clone(), info, init_msg);

        let amount1 = Uint128::from(8899999999988889u128);
        let amount2 = Uint128::from(14487875351811111u128);
        let amount3 = Uint128::from(1100000000000000u128);

        let rewards = Uint128::new(677101666827000000u128);

        let all_balance = amount1 + amount2 + amount3;

        let global_index = Decimal::from_ratio(rewards, all_balance);
        STATE
            .save(
                deps.as_mut().storage,
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
        HOLDERS
            .save(
                deps.storage.borrow_mut(),
                &Addr::unchecked("addr0000"),
                &holder,
            )
            .unwrap();

        let holder = Holder {
            balance: amount2,
            index: Decimal::from_str("0").unwrap(),
            pending_rewards: Decimal::from_str("0").unwrap(),
        };
        HOLDERS
            .save(
                deps.storage.borrow_mut(),
                &Addr::unchecked("addr0001"),
                &holder,
            )
            .unwrap();

        let holder = Holder {
            balance: amount3,
            index: Decimal::from_str("0").unwrap(),
            pending_rewards: Decimal::from_str("0").unwrap(),
        };
        HOLDERS
            .save(
                deps.storage.borrow_mut(),
                &Addr::unchecked("addr0002"),
                &holder,
            )
            .unwrap();

        let msg = ExecuteMsg::ClaimRewards { recipient: None };
        let info = mock_info("addr0000", &[]);
        execute(deps.as_mut(), env.clone(), info, msg).unwrap();

        let msg = ExecuteMsg::ClaimRewards { recipient: None };
        let info = mock_info("addr0001", &[]);
        execute(deps.as_mut(), env.clone(), info, msg).unwrap();

        let msg = ExecuteMsg::ClaimRewards { recipient: None };
        let info = mock_info("addr0002", &[]);
        execute(deps.as_mut(), env.clone(), info, msg).unwrap();

        let res = query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::RewardIndex {
                cw20_addr: MOCK_CW20_CONTRACT_ADDR.into_string(),
            },
        )
        .unwrap();
        let state_response: RewardIndexResponse = from_binary(&res).unwrap();
        assert_eq!(
            state_response,
            RewardIndexResponse {
                global_index,
                total_balance: all_balance,
                prev_reward_balance: Uint128::new(1)
            }
        );
        let res = query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::Holder {
                address: "addr0000".to_string(),
            },
        )
        .unwrap();

        let holder_response: HolderResponse = from_binary(&res).unwrap();
        assert_eq!(
            holder_response,
            HolderResponse {
                address: "addr0000".to_string(),
                balance: amount1,
                index: global_index,
                pending_rewards: Decimal::from_str("0.212799238975421283").unwrap(),
            }
        );

        let res = query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::Holder {
                address: "addr0001".to_string(),
            },
        )
        .unwrap();
        let holder_response: HolderResponse = from_binary(&res).unwrap();
        assert_eq!(
            holder_response,
            HolderResponse {
                address: "addr0001".to_string(),
                balance: amount2,
                index: global_index,
                pending_rewards: Decimal::from_str("0.078595712259178717").unwrap(),
            }
        );

        let res = query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::Holder {
                address: "addr0002".to_string(),
            },
        )
        .unwrap();
        let holder_response: HolderResponse = from_binary(&res).unwrap();
        assert_eq!(
            holder_response,
            HolderResponse {
                address: "addr0002".to_string(),
                balance: amount3,
                index: global_index,
                pending_rewards: Decimal::from_str("0.701700000000000000").unwrap(),
            }
        );
    }

    #[test]
    pub fn proper_calculate_rewards() {
        let global_index = Decimal::from_ratio(Uint128::new(9), Uint128::new(100));
        let user_index = Decimal::zero();
        let user_balance = Uint128::new(1000);
        let reward = calculate_decimal_rewards(global_index, user_index, user_balance).unwrap();
        assert_eq!(reward.to_string(), "90");
    }

    #[test]
    pub fn proper_get_decimals() {
        let global_index = Decimal::from_ratio(Uint128::new(9999999), Uint128::new(100000000));
        let user_index = Decimal::zero();
        let user_balance = Uint128::new(10);
        let reward = get_decimals(
            calculate_decimal_rewards(global_index, user_index, user_balance).unwrap(),
        )
        .unwrap();
        assert_eq!(reward.to_string(), "0.9999999");
    }
}
