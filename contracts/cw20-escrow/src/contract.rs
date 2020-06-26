use cosmwasm_std::{
    log, to_binary, Api, BankMsg, Binary, Coin, CosmosMsg, Env, Extern, HandleResponse, HumanAddr,
    InitResponse, Querier, StdError, StdResult, Storage, WasmMsg,
};
use cw20::Cw20HandleMsg;

use crate::msg::{
    CreateMsg, Cw20CoinHuman, DetailsResponse, HandleMsg, InitMsg, ListResponse, QueryMsg,
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
        HandleMsg::Refund { id } => try_refund(deps, env, id),
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
    if amount.len() == 0 {
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

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use cosmwasm_std::testing::{mock_dependencies, mock_env};
//     use cosmwasm_std::{coins, from_binary, CosmosMsg, StdError, WasmMsg};
//
//     const CANONICAL_LENGTH: usize = 20;
//
//     fn get_balance<S: Storage, A: Api, Q: Querier, T: Into<HumanAddr>>(
//         deps: &Extern<S, A, Q>,
//         address: T,
//     ) -> Uint128 {
//         query_balance(&deps, address.into()).unwrap().balance
//     }
//
//     // this will set up the init for other tests
//     fn do_init<S: Storage, A: Api, Q: Querier>(
//         deps: &mut Extern<S, A, Q>,
//         addr: &HumanAddr,
//         amount: Uint128,
//     ) -> MetaResponse {
//         let init_msg = InitMsg {
//             name: "Auto Gen".to_string(),
//             symbol: "AUTO".to_string(),
//             decimals: 3,
//             initial_balances: vec![InitialBalance {
//                 address: addr.into(),
//                 amount,
//             }],
//         };
//         let env = mock_env(&deps.api, &HumanAddr("creator".to_string()), &[]);
//         let res = init(deps, env, init_msg).unwrap();
//         assert_eq!(0, res.messages.len());
//
//         let meta = query_meta(&deps).unwrap();
//         assert_eq!(
//             meta,
//             MetaResponse {
//                 name: "Auto Gen".to_string(),
//                 symbol: "AUTO".to_string(),
//                 decimals: 3,
//                 total_supply: amount,
//             }
//         );
//         meta
//     }
//
//     #[test]
//     fn proper_initialization() {
//         let mut deps = mock_dependencies(CANONICAL_LENGTH, &[]);
//         let amount = Uint128::from(11223344u128);
//         let init_msg = InitMsg {
//             name: "Cash Token".to_string(),
//             symbol: "CASH".to_string(),
//             decimals: 9,
//             initial_balances: vec![InitialBalance {
//                 address: HumanAddr("addr0000".to_string()),
//                 amount,
//             }],
//         };
//         let env = mock_env(&deps.api, &HumanAddr("creator".to_string()), &[]);
//         let res = init(&mut deps, env, init_msg).unwrap();
//         assert_eq!(0, res.messages.len());
//
//         assert_eq!(
//             query_meta(&deps).unwrap(),
//             MetaResponse {
//                 name: "Cash Token".to_string(),
//                 symbol: "CASH".to_string(),
//                 decimals: 9,
//                 total_supply: amount,
//             }
//         );
//         assert_eq!(get_balance(&deps, "addr0000"), 11223344u128.into());
//     }
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
