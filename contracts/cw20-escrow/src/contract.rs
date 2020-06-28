use cosmwasm_std::{
    from_binary, log, to_binary, Api, BankMsg, Binary, Coin, CosmosMsg, Env, Extern,
    HandleResponse, HumanAddr, InitResponse, Querier, StdError, StdResult, Storage, WasmMsg,
};
use cw20::{Cw20HandleMsg, Cw20ReceiveMsg};

use crate::msg::{
    CreateMsg, Cw20CoinHuman, DetailsResponse, HandleMsg, InitMsg, ListResponse, QueryMsg,
    ReceiveMsg,
};
use crate::state::{all_escrow_ids, escrows, escrows_read, Cw20Coin, Escrow, PREFIX_ESCROW};
use cosmwasm_storage::prefixed;

pub fn init<S: Storage, A: Api, Q: Querier>(
    _deps: &mut Extern<S, A, Q>,
    _env: Env,
    _msg: InitMsg,
) -> StdResult<InitResponse> {
    // no setup
    Ok(InitResponse::default())
}

pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> StdResult<HandleResponse> {
    match msg {
        HandleMsg::Create(msg) => try_create(deps, env, msg),
        HandleMsg::Approve { id } => try_approve(deps, env, id),
        HandleMsg::TopUp { id } => try_top_up(deps, env, id),
        HandleMsg::Refund { id } => try_refund(deps, env, id),
        HandleMsg::Receive(msg) => try_receive(deps, env, msg),
    }
}

pub fn try_receive<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    wrapper: Cw20ReceiveMsg,
) -> StdResult<HandleResponse> {
    let msg: ReceiveMsg = match wrapper.msg {
        Some(bin) => from_binary(&bin),
        None => Err(StdError::parse_err("ReceiveMsg", "no data")),
    }?;
    // TODO: assert the sending token address is valid (contract not external account)
    let token = Cw20Coin {
        address: env.message.sender,
        amount: wrapper.amount,
    };
    match msg {
        ReceiveMsg::Create(create) => try_cw20_create(deps, wrapper.sender, token, create),
        ReceiveMsg::TopUp { id } => try_cw20_top_up(deps, wrapper.sender, token, id),
    }
}

pub fn try_create<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: CreateMsg,
) -> StdResult<HandleResponse> {
    if env.message.sent_funds.is_empty() {
        return Err(StdError::generic_err(
            "You must send some coins to create an escrow",
        ));
    }

    let escrow = Escrow {
        arbiter: deps.api.canonical_address(&msg.arbiter)?,
        recipient: deps.api.canonical_address(&msg.recipient)?,
        source: env.message.sender,
        end_height: msg.end_height,
        end_time: msg.end_time,
        // there are native coins sent with the message
        native_balance: env.message.sent_funds,
        cw20_balance: vec![],
    };

    // try to store it, fail if the id was already in use
    escrows(&mut deps.storage).update(msg.id.as_bytes(), |existing| match existing {
        None => Ok(escrow),
        Some(_) => Err(StdError::generic_err("escrow id already in use")),
    })?;

    let mut res = HandleResponse::default();
    res.log = vec![log("action", "create"), log("id", msg.id)];
    Ok(res)
}

pub fn try_cw20_create<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    sender: HumanAddr,
    token: Cw20Coin,
    msg: CreateMsg,
) -> StdResult<HandleResponse> {
    let escrow = Escrow {
        arbiter: deps.api.canonical_address(&msg.arbiter)?,
        recipient: deps.api.canonical_address(&msg.recipient)?,
        source: deps.api.canonical_address(&sender)?,
        end_height: msg.end_height,
        end_time: msg.end_time,
        // there are native coins sent with the message
        native_balance: vec![],
        cw20_balance: vec![token],
    };

    // try to store it, fail if the id was already in use
    escrows(&mut deps.storage).update(msg.id.as_bytes(), |existing| match existing {
        None => Ok(escrow),
        Some(_) => Err(StdError::generic_err("escrow id already in use")),
    })?;

    let mut res = HandleResponse::default();
    res.log = vec![log("action", "create"), log("id", msg.id)];
    Ok(res)
}

pub fn try_top_up<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    id: String,
) -> StdResult<HandleResponse> {
    // this fails is no escrow there
    let mut escrow = escrows_read(&deps.storage).load(id.as_bytes())?;

    // combine these two
    add_tokens(&mut escrow.native_balance, env.message.sent_funds);
    // and save
    escrows(&mut deps.storage).save(id.as_bytes(), &escrow)?;

    let mut res = HandleResponse::default();
    res.log = vec![log("action", "top_up"), log("id", id)];
    Ok(res)
}

pub fn try_cw20_top_up<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    _sender: HumanAddr,
    token: Cw20Coin,
    id: String,
) -> StdResult<HandleResponse> {
    // this fails is no escrow there
    let mut escrow = escrows_read(&deps.storage).load(id.as_bytes())?;

    // combine these two
    add_cw20_token(&mut escrow.cw20_balance, token);
    // and save
    escrows(&mut deps.storage).save(id.as_bytes(), &escrow)?;

    let mut res = HandleResponse::default();
    res.log = vec![log("action", "top_up"), log("id", id)];
    Ok(res)
}

pub fn try_approve<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    id: String,
) -> StdResult<HandleResponse> {
    // this fails is no escrow there
    let escrow = escrows_read(&deps.storage).load(id.as_bytes())?;

    if env.message.sender != escrow.arbiter {
        Err(StdError::unauthorized())
    } else if escrow.is_expired(&env) {
        Err(StdError::generic_err("escrow expired"))
    } else {
        // we delete the escrow (TODO: expose this in Bucket for simpler API)
        prefixed(PREFIX_ESCROW, &mut deps.storage).remove(id.as_bytes());

        let rcpt = deps.api.human_address(&escrow.recipient)?;
        let contract = deps.api.human_address(&env.contract.address)?;

        // send all tokens out
        let mut messages = send_native_tokens(&contract, &rcpt, escrow.native_balance);
        let mut cw20_send = send_cw20_tokens(&deps.api, &rcpt, escrow.cw20_balance)?;
        messages.append(&mut cw20_send);

        let log = vec![log("action", "approve"), log("id", id), log("to", rcpt)];
        Ok(HandleResponse {
            messages,
            log,
            data: None,
        })
    }
}

pub fn try_refund<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    id: String,
) -> StdResult<HandleResponse> {
    // this fails is no escrow there
    let escrow = escrows_read(&deps.storage).load(id.as_bytes())?;

    // the arbiter can send anytime OR anyone can send after expiration
    if !escrow.is_expired(&env) && env.message.sender != escrow.arbiter {
        Err(StdError::unauthorized())
    } else {
        // we delete the escrow (TODO: expose this in Bucket for simpler API)
        prefixed(PREFIX_ESCROW, &mut deps.storage).remove(id.as_bytes());

        let rcpt = deps.api.human_address(&escrow.source)?;
        let contract = deps.api.human_address(&env.contract.address)?;

        // send all tokens out
        let mut messages = send_native_tokens(&contract, &rcpt, escrow.native_balance);
        let mut cw20_send = send_cw20_tokens(&deps.api, &rcpt, escrow.cw20_balance)?;
        messages.append(&mut cw20_send);

        let log = vec![log("action", "refund"), log("id", id), log("to", rcpt)];
        Ok(HandleResponse {
            messages,
            log,
            data: None,
        })
    }
}

fn send_native_tokens(from: &HumanAddr, to: &HumanAddr, amount: Vec<Coin>) -> Vec<CosmosMsg> {
    if amount.is_empty() {
        vec![]
    } else {
        vec![BankMsg::Send {
            from_address: from.into(),
            to_address: to.into(),
            amount,
        }
        .into()]
    }
}

fn send_cw20_tokens<A: Api>(
    api: &A,
    to: &HumanAddr,
    coins: Vec<Cw20Coin>,
) -> StdResult<Vec<CosmosMsg>> {
    coins
        .into_iter()
        .map(|c| {
            let msg = Cw20HandleMsg::Transfer {
                recipient: to.into(),
                amount: c.amount,
            };
            let exec = WasmMsg::Execute {
                contract_addr: api.human_address(&c.address)?,
                msg: to_binary(&msg)?,
                send: vec![],
            };
            Ok(exec.into())
        })
        .collect()
}

fn add_tokens(store: &mut Vec<Coin>, add: Vec<Coin>) {
    for token in add {
        let index = store.iter().enumerate().find_map(|(i, exist)| {
            if exist.denom == token.denom {
                Some(i)
            } else {
                None
            }
        });
        match index {
            Some(idx) => store[idx].amount += token.amount,
            None => store.push(token),
        }
    }
}

fn add_cw20_token(store: &mut Vec<Cw20Coin>, token: Cw20Coin) {
    let index = store.iter().enumerate().find_map(|(i, exist)| {
        if exist.address == token.address {
            Some(i)
        } else {
            None
        }
    });
    match index {
        Some(idx) => store[idx].amount += token.amount,
        None => store.push(token),
    }
}

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::List {} => to_binary(&query_list(deps)?),
        QueryMsg::Details { id } => to_binary(&query_details(deps, id)?),
    }
}

fn query_details<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    id: String,
) -> StdResult<DetailsResponse> {
    let escrow = escrows_read(&deps.storage).load(id.as_bytes())?;

    // transform tokens
    let cw20_balance: StdResult<Vec<_>> = escrow
        .cw20_balance
        .into_iter()
        .map(|token| {
            Ok(Cw20CoinHuman {
                address: deps.api.human_address(&token.address)?,
                amount: token.amount,
            })
        })
        .collect();

    let details = DetailsResponse {
        id,
        arbiter: deps.api.human_address(&escrow.arbiter)?,
        recipient: deps.api.human_address(&escrow.recipient)?,
        source: deps.api.human_address(&escrow.source)?,
        end_height: escrow.end_height,
        end_time: escrow.end_time,
        native_balance: escrow.native_balance,
        cw20_balance: cw20_balance?,
    };
    Ok(details)
}

fn query_list<S: Storage, A: Api, Q: Querier>(deps: &Extern<S, A, Q>) -> StdResult<ListResponse> {
    Ok(ListResponse {
        escrows: all_escrow_ids(&deps.storage)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, MOCK_CONTRACT_ADDR};
    use cosmwasm_std::{coin, coins, CanonicalAddr, CosmosMsg, StdError, Uint128};

    const CANONICAL_LENGTH: usize = 20;

    #[test]
    fn happy_path_native() {
        let mut deps = mock_dependencies(CANONICAL_LENGTH, &[]);

        // init an empty contract
        let init_msg = InitMsg {};
        let env = mock_env(&deps.api, &HumanAddr::from("anyone"), &[]);
        let res = init(&mut deps, env, init_msg).unwrap();
        assert_eq!(0, res.messages.len());

        // create an escrow
        let create = CreateMsg {
            id: "foobar".to_string(),
            arbiter: HumanAddr::from("arbitrate"),
            recipient: HumanAddr::from("recd"),
            end_time: None,
            end_height: None,
        };
        let sender = HumanAddr::from("source");
        let balance = coins(100, "tokens");
        let env = mock_env(&deps.api, &sender, &balance);
        let res = handle(&mut deps, env, HandleMsg::Create(create.clone())).unwrap();
        assert_eq!(0, res.messages.len());
        assert_eq!(log("action", "create"), res.log[0]);

        // approve it
        let id = create.id.clone();
        let env = mock_env(&deps.api, &create.arbiter, &[]);
        let res = handle(&mut deps, env, HandleMsg::Approve { id }).unwrap();
        assert_eq!(1, res.messages.len());
        assert_eq!(log("action", "approve"), res.log[0]);
        assert_eq!(
            res.messages[0],
            CosmosMsg::Bank(BankMsg::Send {
                from_address: HumanAddr::from(MOCK_CONTRACT_ADDR),
                to_address: create.recipient,
                amount: balance,
            })
        );

        // second attempt fails (not found)
        let id = create.id.clone();
        let env = mock_env(&deps.api, &create.arbiter, &[]);
        let res = handle(&mut deps, env, HandleMsg::Approve { id });
        match res.unwrap_err() {
            StdError::NotFound { .. } => {}
            e => panic!("Expected NotFound, got {}", e),
        }
    }

    #[test]
    fn happy_path_cw20() {
        let mut deps = mock_dependencies(CANONICAL_LENGTH, &[]);

        // init an empty contract
        let init_msg = InitMsg {};
        let env = mock_env(&deps.api, &HumanAddr::from("anyone"), &[]);
        let res = init(&mut deps, env, init_msg).unwrap();
        assert_eq!(0, res.messages.len());

        // create an escrow
        let create = CreateMsg {
            id: "foobar".to_string(),
            arbiter: HumanAddr::from("arbitrate"),
            recipient: HumanAddr::from("recd"),
            end_time: None,
            end_height: None,
        };
        let receive = Cw20ReceiveMsg {
            sender: HumanAddr::from("source"),
            amount: Uint128(100),
            msg: Some(to_binary(&HandleMsg::Create(create.clone())).unwrap()),
        };
        let token_contract = HumanAddr::from("my-cw20-token");
        let env = mock_env(&deps.api, &token_contract, &[]);
        let res = handle(&mut deps, env, HandleMsg::Receive(receive.clone())).unwrap();
        assert_eq!(0, res.messages.len());
        assert_eq!(log("action", "create"), res.log[0]);

        // approve it
        let id = create.id.clone();
        let env = mock_env(&deps.api, &create.arbiter, &[]);
        let res = handle(&mut deps, env, HandleMsg::Approve { id }).unwrap();
        assert_eq!(1, res.messages.len());
        assert_eq!(log("action", "approve"), res.log[0]);
        let send_msg = Cw20HandleMsg::Transfer {
            recipient: create.recipient,
            amount: receive.amount,
        };
        assert_eq!(
            res.messages[0],
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: token_contract,
                msg: to_binary(&send_msg).unwrap(),
                send: vec![]
            })
        );

        // second attempt fails (not found)
        let id = create.id.clone();
        let env = mock_env(&deps.api, &create.arbiter, &[]);
        let res = handle(&mut deps, env, HandleMsg::Approve { id });
        match res.unwrap_err() {
            StdError::NotFound { .. } => {}
            e => panic!("Expected NotFound, got {}", e),
        }
    }

    #[test]
    fn add_tokens_proper() {
        let mut tokens = vec![];
        add_tokens(&mut tokens, vec![coin(123, "atom"), coin(789, "eth")]);
        add_tokens(&mut tokens, vec![coin(456, "atom"), coin(12, "btc")]);
        assert_eq!(
            tokens,
            vec![coin(579, "atom"), coin(789, "eth"), coin(12, "btc")]
        );
    }

    #[test]
    fn add_cw_tokens_proper() {
        let mut tokens = vec![];
        let bar_token = CanonicalAddr(b"bar_token".to_vec().into());
        let foo_token = CanonicalAddr(b"foo_token".to_vec().into());
        add_cw20_token(
            &mut tokens,
            Cw20Coin {
                address: foo_token.clone(),
                amount: Uint128(12345),
            },
        );
        add_cw20_token(
            &mut tokens,
            Cw20Coin {
                address: bar_token.clone(),
                amount: Uint128(777),
            },
        );
        add_cw20_token(
            &mut tokens,
            Cw20Coin {
                address: foo_token.clone(),
                amount: Uint128(23400),
            },
        );
        assert_eq!(
            tokens,
            vec![
                Cw20Coin {
                    address: foo_token,
                    amount: Uint128(35745)
                },
                Cw20Coin {
                    address: bar_token,
                    amount: Uint128(777)
                }
            ]
        );
    }
}
// #[test]
// fn top_up_mixed_tokens() {
//     let mut deps = mock_dependencies(CANONICAL_LENGTH, &[]);
//
//     // init an empty contract
//     let init_msg = InitMsg {};
//     let env = mock_env(&deps.api, &HumanAddr::from("anyone"), &[]);
//     let res = init(&mut deps, env, init_msg).unwrap();
//     assert_eq!(0, res.messages.len());
//
//     // create an escrow
//     let create = CreateMsg {
//         id: "foobar".to_string(),
//         arbiter: HumanAddr::from("arbitrate"),
//         recipient: HumanAddr::from("recd"),
//         end_time: None,
//         end_height: None,
//     };
//     let receive = Cw20ReceiveMsg {
//         sender: HumanAddr::from("source"),
//         amount: Uint128(100),
//         msg: Some(to_binary(&HandleMsg::Create(create.clone())).unwrap()),
//     };
//     let token_contract = HumanAddr::from("my-cw20-token");
//     let env = mock_env(&deps.api, &token_contract, &[]);
//     let res = handle(&mut deps, env, HandleMsg::Receive(receive.clone())).unwrap();
//     assert_eq!(0, res.messages.len());
//     assert_eq!(log("action", "create"), res.log[0]);
//
//     // approve it
//     let id = create.id.clone();
//     let env = mock_env(&deps.api, &create.arbiter, &[]);
//     let res = handle(&mut deps, env, HandleMsg::Approve { id }).unwrap();
//     assert_eq!(1, res.messages.len());
//     assert_eq!(log("action", "approve"), res.log[0]);
//     let send_msg = Cw20HandleMsg::Transfer {
//         recipient: create.recipient,
//         amount: receive.amount,
//     };
//     assert_eq!(
//         res.messages[0],
//         CosmosMsg::Wasm(WasmMsg::Execute {
//             contract_addr: token_contract,
//             msg: to_binary(&send_msg).unwrap(),
//             send: vec![]
//         })
//     );
//
//     // second attempt fails (not found)
//     let id = create.id.clone();
//     let env = mock_env(&deps.api, &create.arbiter, &[]);
//     let res = handle(&mut deps, env, HandleMsg::Approve { id });
//     match res.unwrap_err() {
//         StdError::NotFound { .. } => {}
//         e => panic!("Expected NotFound, got {}", e),
//     }
// }

//
//     #[test]
//     fn init_multiple_accounts() {
//         let mut deps = mock_dependencies(CANONICAL_LENGTH, &[]);
//         let amount1 = Uint128::from(11223344u128);
//         let addr1 = HumanAddr::from("addr0001");
//         let amount2 = Uint128::from(7890987u128);
//         let addr2 = HumanAddr::from("addr0002");
//         let init_msg = InitMsg {
//             name: "Bash Shell".to_string(),
//             symbol: "BASH".to_string(),
//             decimals: 6,
//             initial_balances: vec![
//                 InitialBalance {
//                     address: addr1.clone(),
//                     amount: amount1,
//                 },
//                 InitialBalance {
//                     address: addr2.clone(),
//                     amount: amount2,
//                 },
//             ],
//         };
//         let env = mock_env(&deps.api, &HumanAddr("creator".to_string()), &[]);
//         let res = init(&mut deps, env, init_msg).unwrap();
//         assert_eq!(0, res.messages.len());
//
//         assert_eq!(
//             query_meta(&deps).unwrap(),
//             MetaResponse {
//                 name: "Bash Shell".to_string(),
//                 symbol: "BASH".to_string(),
//                 decimals: 6,
//                 total_supply: amount1 + amount2,
//             }
//         );
//         assert_eq!(get_balance(&deps, &addr1), amount1);
//         assert_eq!(get_balance(&deps, &addr2), amount2);
//     }
//
//     #[test]
//     fn queries_work() {
//         let mut deps = mock_dependencies(20, &coins(2, "token"));
//         let addr1 = HumanAddr::from("addr0001");
//         let amount1 = Uint128::from(12340000u128);
//
//         let expected = do_init(&mut deps, &addr1, amount1);
//
//         // check meta query
//         let loaded = query_meta(&deps).unwrap();
//         assert_eq!(expected, loaded);
//
//         // check balance query (full)
//         let data = query(
//             &deps,
//             QueryMsg::Balance {
//                 address: addr1.clone(),
//             },
//         )
//         .unwrap();
//         let loaded: BalanceResponse = from_binary(&data).unwrap();
//         assert_eq!(loaded.balance, amount1);
//
//         // check balance query (empty)
//         let data = query(
//             &deps,
//             QueryMsg::Balance {
//                 address: HumanAddr::from("addr0002"),
//             },
//         )
//         .unwrap();
//         let loaded: BalanceResponse = from_binary(&data).unwrap();
//         assert_eq!(loaded.balance, Uint128::zero());
//     }
//
//     #[test]
//     fn transfer() {
//         let mut deps = mock_dependencies(20, &coins(2, "token"));
//         let addr1 = HumanAddr::from("addr0001");
//         let addr2 = HumanAddr::from("addr0002");
//         let amount1 = Uint128::from(12340000u128);
//         let transfer = Uint128::from(76543u128);
//         let too_much = Uint128::from(12340321u128);
//
//         do_init(&mut deps, &addr1, amount1);
//
//         // cannot send more than we have
//         let env = mock_env(&deps.api, addr1.clone(), &[]);
//         let msg = HandleMsg::Transfer {
//             recipient: addr2.clone(),
//             amount: too_much,
//         };
//         let res = handle(&mut deps, env, msg);
//         match res.unwrap_err() {
//             StdError::Underflow { .. } => {}
//             e => panic!("Unexpected error: {}", e),
//         }
//
//         // cannot send from empty account
//         let env = mock_env(&deps.api, addr2.clone(), &[]);
//         let msg = HandleMsg::Transfer {
//             recipient: addr1.clone(),
//             amount: transfer,
//         };
//         let res = handle(&mut deps, env, msg);
//         match res.unwrap_err() {
//             StdError::Underflow { .. } => {}
//             e => panic!("Unexpected error: {}", e),
//         }
//
//         // valid transfer
//         let env = mock_env(&deps.api, addr1.clone(), &[]);
//         let msg = HandleMsg::Transfer {
//             recipient: addr2.clone(),
//             amount: transfer,
//         };
//         let res = handle(&mut deps, env, msg).unwrap();
//         assert_eq!(res.messages.len(), 0);
//
//         let remainder = (amount1 - transfer).unwrap();
//         assert_eq!(get_balance(&deps, &addr1), remainder);
//         assert_eq!(get_balance(&deps, &addr2), transfer);
//         assert_eq!(query_meta(&deps).unwrap().total_supply, amount1);
//     }
//
//     #[test]
//     fn burn() {
//         let mut deps = mock_dependencies(20, &coins(2, "token"));
//         let addr1 = HumanAddr::from("addr0001");
//         let amount1 = Uint128::from(12340000u128);
//         let burn = Uint128::from(76543u128);
//         let too_much = Uint128::from(12340321u128);
//
//         do_init(&mut deps, &addr1, amount1);
//
//         // cannot burn more than we have
//         let env = mock_env(&deps.api, addr1.clone(), &[]);
//         let msg = HandleMsg::Burn { amount: too_much };
//         let res = handle(&mut deps, env, msg);
//         match res.unwrap_err() {
//             StdError::Underflow { .. } => {}
//             e => panic!("Unexpected error: {}", e),
//         }
//         assert_eq!(query_meta(&deps).unwrap().total_supply, amount1);
//
//         // valid burn reduces total supply
//         let env = mock_env(&deps.api, addr1.clone(), &[]);
//         let msg = HandleMsg::Burn { amount: burn };
//         let res = handle(&mut deps, env, msg).unwrap();
//         assert_eq!(res.messages.len(), 0);
//
//         let remainder = (amount1 - burn).unwrap();
//         assert_eq!(get_balance(&deps, &addr1), remainder);
//         assert_eq!(query_meta(&deps).unwrap().total_supply, remainder);
//     }
//
//     #[test]
//     fn send() {
//         let mut deps = mock_dependencies(20, &coins(2, "token"));
//         let addr1 = HumanAddr::from("addr0001");
//         let contract = HumanAddr::from("addr0002");
//         let amount1 = Uint128::from(12340000u128);
//         let transfer = Uint128::from(76543u128);
//         let too_much = Uint128::from(12340321u128);
//         let send_msg = Binary::from(r#"{"some":123}"#.as_bytes());
//
//         do_init(&mut deps, &addr1, amount1);
//
//         // cannot send more than we have
//         let env = mock_env(&deps.api, addr1.clone(), &[]);
//         let msg = HandleMsg::Send {
//             contract: contract.clone(),
//             amount: too_much,
//             msg: Some(send_msg.clone()),
//         };
//         let res = handle(&mut deps, env, msg);
//         match res.unwrap_err() {
//             StdError::Underflow { .. } => {}
//             e => panic!("Unexpected error: {}", e),
//         }
//
//         // valid transfer
//         let env = mock_env(&deps.api, addr1.clone(), &[]);
//         let msg = HandleMsg::Send {
//             contract: contract.clone(),
//             amount: transfer,
//             msg: Some(send_msg.clone()),
//         };
//         let res = handle(&mut deps, env, msg).unwrap();
//         assert_eq!(res.messages.len(), 1);
//
//         // ensure proper send message sent
//         // this is the message we want delivered to the other side
//         let binary_msg = Cw20ReceiveMsg {
//             sender: addr1.clone(),
//             amount: transfer,
//             msg: Some(send_msg),
//         }
//         .into_binary()
//         .unwrap();
//         // and this is how it must be wrapped for the vm to process it
//         assert_eq!(
//             res.messages[0],
//             CosmosMsg::Wasm(WasmMsg::Execute {
//                 contract_addr: contract.clone(),
//                 msg: binary_msg,
//                 send: vec![],
//             })
//         );
//
//         // ensure balance is properly transfered
//         let remainder = (amount1 - transfer).unwrap();
//         assert_eq!(get_balance(&deps, &addr1), remainder);
//         assert_eq!(get_balance(&deps, &contract), transfer);
//         assert_eq!(query_meta(&deps).unwrap().total_supply, amount1);
//     }
// }
