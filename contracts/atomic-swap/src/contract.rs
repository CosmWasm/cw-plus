use cosmwasm_std::{
    log, Api, BankMsg, CosmosMsg, Env, Extern, HandleResponse, InitResponse, Querier, StdError,
    StdResult, Storage,
};
use sha2::{Digest, Sha256};

use cw2::{set_contract_version, ContractVersion};

use crate::msg::{HandleMsg, InitMsg};
use crate::state::{atomic_swap, atomic_swap_read, AtomicSwap};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:atomic-swap";
const CONTRACT_VERSION: &str = "v0.1.0";

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    // Ensure this is 32 bytes hex-encoded
    let _ = parse_hex_32(&msg.hash)?;

    let state = AtomicSwap {
        hash: msg.hash,
        recipient: deps.api.canonical_address(&msg.recipient)?,
        source: deps.api.canonical_address(&env.message.sender)?,
        end_height: msg.end_height,
        end_time: msg.end_time,
    };

    if state.is_expired(&env) {
        Err(StdError::generic_err("Creating expired atomic swap"))
    } else {
        let version = ContractVersion {
            contract: CONTRACT_NAME.to_string(),
            version: CONTRACT_VERSION.to_string(),
        };
        set_contract_version(&mut deps.storage, &version)?;
        atomic_swap(&mut deps.storage).save(&state)?;
        Ok(InitResponse::default())
    }
}

pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> StdResult<HandleResponse> {
    match msg {
        HandleMsg::Release { preimage } => try_release(deps, env, preimage),
        HandleMsg::Refund {} => try_refund(deps, env),
    }
}

pub fn try_release<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    preimage: String,
) -> StdResult<HandleResponse> {
    let state = atomic_swap_read(&deps.storage).load()?;
    if state.is_expired(&env) {
        return Err(StdError::generic_err("Atomic swap expired"));
    }

    let expected = parse_hex_32(&state.hash)?;
    let hash = Sha256::digest(&parse_hex_32(&preimage)?);
    if hash.as_slice() != expected.as_slice() {
        return Err(StdError::generic_err("Invalid preimage"));
    }

    // We delete the swap
    atomic_swap(&mut deps.storage).remove();

    let rcpt = deps.api.human_address(&state.recipient)?;

    let msg = vec![CosmosMsg::Bank(BankMsg::Send {
        from_address: env.contract.address,
        to_address: rcpt.clone(),
        amount: env.message.sent_funds,
    })];
    Ok(HandleResponse {
        messages: msg,
        log: vec![
            log("action", "release"),
            log("preimage", preimage),
            log("to", rcpt),
        ],
        data: None,
    })
}

pub fn try_refund<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
) -> StdResult<HandleResponse> {
    let state = atomic_swap_read(&deps.storage).load()?;
    // Anyone can try to refund, as long as the contract is expired
    if !state.is_expired(&env) {
        return Err(StdError::generic_err("Atomic swap not yet expired"));
    }

    // We delete the swap
    atomic_swap(&mut deps.storage).remove();

    let src = deps.api.human_address(&state.source)?;

    let msg = vec![CosmosMsg::Bank(BankMsg::Send {
        from_address: env.contract.address,
        to_address: src.clone(),
        amount: env.message.sent_funds,
    })];
    Ok(HandleResponse {
        messages: msg,
        log: vec![log("action", "refund"), log("to", src)],
        data: None,
    })
}

fn parse_hex_32(data: &str) -> StdResult<Vec<u8>> {
    match hex::decode(data) {
        Ok(bin) => {
            if bin.len() == 32 {
                Ok(bin)
            } else {
                Err(StdError::generic_err("Hash must be 64 characters"))
            }
        }
        Err(e) => Err(StdError::generic_err(format!(
            "Error parsing hash: {}",
            e.to_string()
        ))),
    }
}

/*
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
*/

/*
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
*/

/*
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
*/

/*
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
*/

/*
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

    let cw20_whitelist = escrow.human_whitelist(&deps.api)?;

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
        cw20_whitelist,
    };
    Ok(details)
}

fn query_list<S: Storage, A: Api, Q: Querier>(deps: &Extern<S, A, Q>) -> StdResult<ListResponse> {
    Ok(ListResponse {
        escrows: all_escrow_ids(&deps.storage)?,
    })
}
*/

#[cfg(test)]
mod tests {
    /*
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, MOCK_CONTRACT_ADDR};
    use cosmwasm_std::{coin, coins, CanonicalAddr, CosmosMsg, StdError, Uint128};

    const CANONICAL_LENGTH: usize = 20;

    #[test]
    fn happy_path_native() {
        let mut deps = mock_dependencies(CANONICAL_LENGTH, &[]);

        // init an empty contract
        let init_msg = InitMsg {};
        let env = mock_env(&HumanAddr::from("anyone"), &[]);
        let res = init(&mut deps, env, init_msg).unwrap();
        assert_eq!(0, res.messages.len());

        // create an escrow
        let create = CreateMsg {
            id: "foobar".to_string(),
            arbiter: HumanAddr::from("arbitrate"),
            recipient: HumanAddr::from("recd"),
            end_time: None,
            end_height: Some(123456),
            cw20_whitelist: None,
        };
        let sender = HumanAddr::from("source");
        let balance = coins(100, "tokens");
        let env = mock_env(&sender, &balance);
        let res = handle(&mut deps, env, HandleMsg::Create(create.clone())).unwrap();
        assert_eq!(0, res.messages.len());
        assert_eq!(log("action", "create"), res.log[0]);

        // ensure the details is what we expect
        let details = query_details(&deps, "foobar".to_string()).unwrap();
        assert_eq!(
            details,
            DetailsResponse {
                id: "foobar".to_string(),
                arbiter: HumanAddr::from("arbitrate"),
                recipient: HumanAddr::from("recd"),
                source: HumanAddr::from("source"),
                end_height: Some(123456),
                end_time: None,
                native_balance: balance.clone(),
                cw20_balance: vec![],
                cw20_whitelist: vec![],
            }
        );

        // approve it
        let id = create.id.clone();
        let env = mock_env(&create.arbiter, &[]);
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
        let env = mock_env(&create.arbiter, &[]);
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
        let env = mock_env(&HumanAddr::from("anyone"), &[]);
        let res = init(&mut deps, env, init_msg).unwrap();
        assert_eq!(0, res.messages.len());

        // create an escrow
        let create = CreateMsg {
            id: "foobar".to_string(),
            arbiter: HumanAddr::from("arbitrate"),
            recipient: HumanAddr::from("recd"),
            end_time: None,
            end_height: None,
            cw20_whitelist: Some(vec![HumanAddr::from("other-token")]),
        };
        let receive = Cw20ReceiveMsg {
            sender: HumanAddr::from("source"),
            amount: Uint128(100),
            msg: Some(to_binary(&HandleMsg::Create(create.clone())).unwrap()),
        };
        let token_contract = HumanAddr::from("my-cw20-token");
        let env = mock_env(&token_contract, &[]);
        let res = handle(&mut deps, env, HandleMsg::Receive(receive.clone())).unwrap();
        assert_eq!(0, res.messages.len());
        assert_eq!(log("action", "create"), res.log[0]);

        // ensure the whitelist is what we expect
        let details = query_details(&deps, "foobar".to_string()).unwrap();
        assert_eq!(
            details,
            DetailsResponse {
                id: "foobar".to_string(),
                arbiter: HumanAddr::from("arbitrate"),
                recipient: HumanAddr::from("recd"),
                source: HumanAddr::from("source"),
                end_height: None,
                end_time: None,
                native_balance: vec![],
                cw20_balance: vec![Cw20CoinHuman {
                    address: HumanAddr::from("my-cw20-token"),
                    amount: Uint128(100)
                }],
                cw20_whitelist: vec![
                    HumanAddr::from("other-token"),
                    HumanAddr::from("my-cw20-token")
                ],
            }
        );

        // approve it
        let id = create.id.clone();
        let env = mock_env(&create.arbiter, &[]);
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
        let env = mock_env(&create.arbiter, &[]);
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

    #[test]
    fn top_up_mixed_tokens() {
        let mut deps = mock_dependencies(CANONICAL_LENGTH, &[]);

        // init an empty contract
        let init_msg = InitMsg {};
        let env = mock_env(&HumanAddr::from("anyone"), &[]);
        let res = init(&mut deps, env, init_msg).unwrap();
        assert_eq!(0, res.messages.len());

        // only accept these tokens
        let whitelist = vec![HumanAddr::from("bar_token"), HumanAddr::from("foo_token")];

        // create an escrow with 2 native tokens
        let create = CreateMsg {
            id: "foobar".to_string(),
            arbiter: HumanAddr::from("arbitrate"),
            recipient: HumanAddr::from("recd"),
            end_time: None,
            end_height: None,
            cw20_whitelist: Some(whitelist),
        };
        let sender = HumanAddr::from("source");
        let balance = vec![coin(100, "fee"), coin(200, "stake")];
        let env = mock_env(&sender, &balance);
        let res = handle(&mut deps, env, HandleMsg::Create(create.clone())).unwrap();
        assert_eq!(0, res.messages.len());
        assert_eq!(log("action", "create"), res.log[0]);

        // top it up with 2 more native tokens
        let extra_native = vec![coin(250, "random"), coin(300, "stake")];
        let env = mock_env(&sender, &extra_native);
        let top_up = HandleMsg::TopUp {
            id: create.id.clone(),
        };
        let res = handle(&mut deps, env, top_up).unwrap();
        assert_eq!(0, res.messages.len());
        assert_eq!(log("action", "top_up"), res.log[0]);

        // top up with one foreign token
        let bar_token = HumanAddr::from("bar_token");
        let base = TopUp {
            id: create.id.clone(),
        };
        let top_up = HandleMsg::Receive(Cw20ReceiveMsg {
            sender: HumanAddr::from("random"),
            amount: Uint128(7890),
            msg: Some(to_binary(&base).unwrap()),
        });
        let env = mock_env(&bar_token, &[]);
        let res = handle(&mut deps, env, top_up).unwrap();
        assert_eq!(0, res.messages.len());
        assert_eq!(log("action", "top_up"), res.log[0]);

        // top with a foreign token not on the whitelist
        // top up with one foreign token
        let baz_token = HumanAddr::from("baz_token");
        let base = TopUp {
            id: create.id.clone(),
        };
        let top_up = HandleMsg::Receive(Cw20ReceiveMsg {
            sender: HumanAddr::from("random"),
            amount: Uint128(7890),
            msg: Some(to_binary(&base).unwrap()),
        });
        let env = mock_env(&baz_token, &[]);
        let res = handle(&mut deps, env, top_up);
        match res.unwrap_err() {
            StdError::GenericErr { msg, .. } => {
                assert_eq!(msg, "Only accepts tokens on the cw20_whitelist")
            }
            e => panic!("Unexpected error: {}", e),
        }

        // top up with second foreign token
        let foo_token = HumanAddr::from("foo_token");
        let base = TopUp {
            id: create.id.clone(),
        };
        let top_up = HandleMsg::Receive(Cw20ReceiveMsg {
            sender: HumanAddr::from("random"),
            amount: Uint128(888),
            msg: Some(to_binary(&base).unwrap()),
        });
        let env = mock_env(&foo_token, &[]);
        let res = handle(&mut deps, env, top_up).unwrap();
        assert_eq!(0, res.messages.len());
        assert_eq!(log("action", "top_up"), res.log[0]);

        // approve it
        let id = create.id.clone();
        let env = mock_env(&create.arbiter, &[]);
        let res = handle(&mut deps, env, HandleMsg::Approve { id }).unwrap();
        assert_eq!(log("action", "approve"), res.log[0]);
        assert_eq!(3, res.messages.len());

        // first message releases all native coins
        assert_eq!(
            res.messages[0],
            CosmosMsg::Bank(BankMsg::Send {
                from_address: HumanAddr::from(MOCK_CONTRACT_ADDR),
                to_address: create.recipient.clone(),
                amount: vec![coin(100, "fee"), coin(500, "stake"), coin(250, "random")],
            })
        );

        // second one release bar cw20 token
        let send_msg = Cw20HandleMsg::Transfer {
            recipient: create.recipient.clone(),
            amount: Uint128(7890),
        };
        assert_eq!(
            res.messages[1],
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: bar_token,
                msg: to_binary(&send_msg).unwrap(),
                send: vec![]
            })
        );

        // third one release foo cw20 token
        let send_msg = Cw20HandleMsg::Transfer {
            recipient: create.recipient.clone(),
            amount: Uint128(888),
        };
        assert_eq!(
            res.messages[2],
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: foo_token,
                msg: to_binary(&send_msg).unwrap(),
                send: vec![]
            })
        );
    }
    */
}
