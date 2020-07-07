use schemars::JsonSchema;
use std::fmt;

use cosmwasm_std::{
    log, to_binary, Api, Binary, CanonicalAddr, CosmosMsg, Empty, Env, Extern, HandleResponse,
    HumanAddr, InitResponse, Querier, StdError, StdResult, Storage,
};

use crate::msg::{ConfigResponse, HandleMsg, InitMsg, QueryMsg};
use crate::state::{config, config_read, Config};

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    _env: Env,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    let cfg = Config {
        admins: map_canonical(&deps.api, &msg.admins)?,
        mutable: msg.mutable,
    };
    config(&mut deps.storage).save(&cfg)?;
    Ok(InitResponse::default())
}

fn map_canonical<A: Api>(api: &A, admins: &[HumanAddr]) -> StdResult<Vec<CanonicalAddr>> {
    admins
        .iter()
        .map(|addr| api.canonical_address(addr))
        .collect()
}

fn map_human<A: Api>(api: &A, admins: &[CanonicalAddr]) -> StdResult<Vec<HumanAddr>> {
    admins.iter().map(|addr| api.human_address(addr)).collect()
}

pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    // Note: implement this function with different type to add support for custom messages
    // and then import the rest of this contract code.
    msg: HandleMsg<Empty>,
) -> StdResult<HandleResponse<Empty>> {
    match msg {
        HandleMsg::Execute { msgs } => handle_execute(deps, env, msgs),
        HandleMsg::Freeze {} => handle_freeze(deps, env),
        HandleMsg::UpdateAdmins { admins } => handle_update_admins(deps, env, admins),
    }
}

pub fn handle_execute<S: Storage, A: Api, Q: Querier, T>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msgs: Vec<CosmosMsg<T>>,
) -> StdResult<HandleResponse<T>>
where
    T: Clone + fmt::Debug + PartialEq + JsonSchema,
{
    let cfg = config_read(&deps.storage).load()?;
    if !cfg.is_admin(&env.message.sender) {
        Err(StdError::unauthorized())
    } else {
        let mut res = HandleResponse::default();
        res.messages = msgs;
        res.log = vec![log("action", "execute")];
        Ok(res)
    }
}

pub fn handle_freeze<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
) -> StdResult<HandleResponse> {
    let mut cfg = config_read(&deps.storage).load()?;
    if !cfg.can_modify(&env.message.sender) {
        Err(StdError::unauthorized())
    } else {
        cfg.mutable = false;
        config(&mut deps.storage).save(&cfg)?;

        let mut res = HandleResponse::default();
        res.log = vec![log("action", "freeze")];
        Ok(res)
    }
}

pub fn handle_update_admins<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    admins: Vec<HumanAddr>,
) -> StdResult<HandleResponse> {
    let mut cfg = config_read(&deps.storage).load()?;
    if !cfg.can_modify(&env.message.sender) {
        Err(StdError::unauthorized())
    } else {
        cfg.admins = map_canonical(&deps.api, &admins)?;
        config(&mut deps.storage).save(&cfg)?;

        let mut res = HandleResponse::default();
        res.log = vec![log("action", "update_admins")];
        Ok(res)
    }
}

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
    }
}

fn query_config<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> StdResult<ConfigResponse> {
    let cfg = config_read(&deps.storage).load()?;
    Ok(ConfigResponse {
        admins: map_human(&deps.api, &cfg.admins)?,
        mutable: cfg.mutable,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::msg::HandleMsg::TopUp;
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
            end_height: Some(123456),
            cw20_whitelist: None,
        };
        let sender = HumanAddr::from("source");
        let balance = coins(100, "tokens");
        let env = mock_env(&deps.api, &sender, &balance);
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
            cw20_whitelist: Some(vec![HumanAddr::from("other-token")]),
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

    #[test]
    fn top_up_mixed_tokens() {
        let mut deps = mock_dependencies(CANONICAL_LENGTH, &[]);

        // init an empty contract
        let init_msg = InitMsg {};
        let env = mock_env(&deps.api, &HumanAddr::from("anyone"), &[]);
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
        let env = mock_env(&deps.api, &sender, &balance);
        let res = handle(&mut deps, env, HandleMsg::Create(create.clone())).unwrap();
        assert_eq!(0, res.messages.len());
        assert_eq!(log("action", "create"), res.log[0]);

        // top it up with 2 more native tokens
        let extra_native = vec![coin(250, "random"), coin(300, "stake")];
        let env = mock_env(&deps.api, &sender, &extra_native);
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
        let env = mock_env(&deps.api, &bar_token, &[]);
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
        let env = mock_env(&deps.api, &baz_token, &[]);
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
        let env = mock_env(&deps.api, &foo_token, &[]);
        let res = handle(&mut deps, env, top_up).unwrap();
        assert_eq!(0, res.messages.len());
        assert_eq!(log("action", "top_up"), res.log[0]);

        // approve it
        let id = create.id.clone();
        let env = mock_env(&deps.api, &create.arbiter, &[]);
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
}
