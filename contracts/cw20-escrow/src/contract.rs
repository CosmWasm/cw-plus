use cosmwasm_std::{
    attr, from_binary, to_binary, Api, BankMsg, Binary, Coin, CosmosMsg, Env, Extern,
    HandleResponse, HumanAddr, InitResponse, Querier, StdError, StdResult, Storage, WasmMsg,
};
use cosmwasm_storage::prefixed;

use cw2::set_contract_version;
use cw20::{Cw20Coin, Cw20CoinHuman, Cw20HandleMsg, Cw20ReceiveMsg};
use cw20_atomic_swap::balance::Balance;

use crate::msg::{
    CreateMsg, DetailsResponse, HandleMsg, InitMsg, ListResponse, QueryMsg, ReceiveMsg,
};
use crate::state::{all_escrow_ids, escrows, escrows_read, Escrow, GenericBalance, PREFIX_ESCROW};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cw20-escrow";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    _env: Env,
    _msg: InitMsg,
) -> StdResult<InitResponse> {
    set_contract_version(&mut deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    // no setup
    Ok(InitResponse::default())
}

pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> StdResult<HandleResponse> {
    match msg {
        HandleMsg::Create(msg) => try_create(
            deps,
            msg,
            Balance::from(env.message.sent_funds),
            &env.message.sender,
        ),
        HandleMsg::Approve { id } => try_approve(deps, env, id),
        HandleMsg::TopUp { id } => try_top_up(deps, id, Balance::from(env.message.sent_funds)),
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
    let balance = Balance::Cw20(Cw20Coin {
        address: deps.api.canonical_address(&env.message.sender)?,
        amount: wrapper.amount,
    });
    match msg {
        ReceiveMsg::Create(msg) => try_create(deps, msg, balance, &wrapper.sender),
        ReceiveMsg::TopUp { id } => try_top_up(deps, id, balance),
    }
}

pub fn try_create<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    msg: CreateMsg,
    balance: Balance,
    sender: &HumanAddr,
) -> StdResult<HandleResponse> {
    if balance.is_empty() {
        return Err(StdError::generic_err("Send some coins to create an escrow"));
    }

    let escrow = Escrow {
        arbiter: deps.api.canonical_address(&msg.arbiter)?,
        recipient: deps.api.canonical_address(&msg.recipient)?,
        source: deps.api.canonical_address(&env.message.sender)?,
        end_height: msg.end_height,
        end_time: msg.end_time,
        // there are native coins sent with the message
        native_balance: env.message.sent_funds,
        cw20_balance: vec![],
        cw20_whitelist: msg.canonical_whitelist(&deps.api)?,
    };

    // try to store it, fail if the id was already in use
    escrows(&mut deps.storage).update(msg.id.as_bytes(), |existing| match existing {
        None => Ok(escrow),
        Some(_) => Err(StdError::generic_err("escrow id already in use")),
    })?;

    let mut res = HandleResponse::default();
    res.attributes = vec![attr("action", "create"), attr("id", msg.id)];
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
    res.attributes = vec![attr("action", "top_up"), attr("id", id)];
    Ok(res)
}

pub fn try_approve<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    id: String,
) -> StdResult<HandleResponse> {
    // this fails is no escrow there
    let escrow = escrows_read(&deps.storage).load(id.as_bytes())?;

    if deps.api.canonical_address(&env.message.sender)? != escrow.arbiter {
        Err(StdError::unauthorized())
    } else if escrow.is_expired(&env) {
        Err(StdError::generic_err("escrow expired"))
    } else {
        // we delete the escrow (TODO: expose this in Bucket for simpler API)
        prefixed(PREFIX_ESCROW, &mut deps.storage).remove(id.as_bytes());

        let rcpt = deps.api.human_address(&escrow.recipient)?;

        // send all tokens out
        let messages = send_tokens(&deps.api, &env.contract.address, &rcpt, &escrow.balance)?;

        let attributes = vec![attr("action", "approve"), attr("id", id), attr("to", rcpt)];
        Ok(HandleResponse {
            messages,
            attributes,
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
    if !escrow.is_expired(&env)
        && deps.api.canonical_address(&env.message.sender)? != escrow.arbiter
    {
        Err(StdError::unauthorized())
    } else {
        // we delete the escrow (TODO: expose this in Bucket for simpler API)
        prefixed(PREFIX_ESCROW, &mut deps.storage).remove(id.as_bytes());

        let rcpt = deps.api.human_address(&escrow.source)?;

        // send all tokens out
        let messages = send_tokens(&deps.api, &env.contract.address, &rcpt, &escrow.balance)?;

        let attributes = vec![attr("action", "refund"), attr("id", id), attr("to", rcpt)];
        Ok(HandleResponse {
            messages,
            attributes,
            data: None,
        })
    }
}

fn send_tokens<A: Api>(
    api: &A,
    from: &HumanAddr,
    to: &HumanAddr,
    balance: &GenericBalance,
) -> StdResult<Vec<CosmosMsg>> {
    let native_balance = &balance.native;
    let mut msgs: Vec<CosmosMsg> = if native_balance.is_empty() {
        vec![]
    } else {
        vec![BankMsg::Send {
            from_address: from.into(),
            to_address: to.into(),
            amount: native_balance.to_vec(),
        }
        .into()]
    };

    let cw20_balance = &balance.cw20;
    let cw20_msgs: StdResult<Vec<_>> = cw20_balance
        .iter()
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
        .collect();
    msgs.append(&mut cw20_msgs?);
    Ok(msgs)
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

    let cw20_whitelist = escrow.human_whitelist(&deps.api)?;

    // transform tokens
    let native_balance = escrow.balance.native;

    let cw20_balance: StdResult<Vec<_>> = escrow
        .balance
        .cw20
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
        native_balance,
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

#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::{mock_dependencies, mock_env, MOCK_CONTRACT_ADDR};
    use cosmwasm_std::{coin, coins, CanonicalAddr, CosmosMsg, StdError, Uint128};

    use crate::msg::HandleMsg::TopUp;

    use super::*;

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
        assert_eq!(attr("action", "create"), res.attributes[0]);

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
        assert_eq!(attr("action", "approve"), res.attributes[0]);
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
        assert_eq!(attr("action", "create"), res.attributes[0]);

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
                    amount: Uint128(100),
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
        assert_eq!(attr("action", "approve"), res.attributes[0]);
        let send_msg = Cw20HandleMsg::Transfer {
            recipient: create.recipient,
            amount: receive.amount,
        };
        assert_eq!(
            res.messages[0],
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: token_contract,
                msg: to_binary(&send_msg).unwrap(),
                send: vec![],
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
        let mut tokens = GenericBalance::default();
        tokens.add_tokens(Balance::from(vec![coin(123, "atom"), coin(789, "eth")]));
        tokens.add_tokens(Balance::from(vec![coin(456, "atom"), coin(12, "btc")]));
        assert_eq!(
            tokens.native,
            vec![coin(579, "atom"), coin(789, "eth"), coin(12, "btc")]
        );
    }

    #[test]
    fn add_cw_tokens_proper() {
        let mut tokens = GenericBalance::default();
        let bar_token = CanonicalAddr(b"bar_token".to_vec().into());
        let foo_token = CanonicalAddr(b"foo_token".to_vec().into());
        tokens.add_tokens(Balance::Cw20(Cw20Coin {
            address: foo_token.clone(),
            amount: Uint128(12345),
        }));
        tokens.add_tokens(Balance::Cw20(Cw20Coin {
            address: bar_token.clone(),
            amount: Uint128(777),
        }));
        tokens.add_tokens(Balance::Cw20(Cw20Coin {
            address: foo_token.clone(),
            amount: Uint128(23400),
        }));
        assert_eq!(
            tokens.cw20,
            vec![
                Cw20Coin {
                    address: foo_token,
                    amount: Uint128(35745),
                },
                Cw20Coin {
                    address: bar_token,
                    amount: Uint128(777),
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
        assert_eq!(attr("action", "create"), res.attributes[0]);

        // top it up with 2 more native tokens
        let extra_native = vec![coin(250, "random"), coin(300, "stake")];
        let env = mock_env(&sender, &extra_native);
        let top_up = HandleMsg::TopUp {
            id: create.id.clone(),
        };
        let res = handle(&mut deps, env, top_up).unwrap();
        assert_eq!(0, res.messages.len());
        assert_eq!(attr("action", "top_up"), res.attributes[0]);

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
        assert_eq!(attr("action", "top_up"), res.attributes[0]);

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
        assert_eq!(attr("action", "top_up"), res.attributes[0]);

        // approve it
        let id = create.id.clone();
        let env = mock_env(&create.arbiter, &[]);
        let res = handle(&mut deps, env, HandleMsg::Approve { id }).unwrap();
        assert_eq!(attr("action", "approve"), res.attributes[0]);
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
                send: vec![],
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
                send: vec![],
            })
        );
    }
}
