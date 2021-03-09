use cosmwasm_std::{
    attr, entry_point, from_binary, to_binary, Binary, Deps, DepsMut, Env, HumanAddr, IbcMsg,
    IbcQuery, MessageInfo, Order, PortIdResponse, Response, StdResult,
};

use cw2::set_contract_version;
use cw20::{Cw20CoinHuman, Cw20ReceiveMsg};

use crate::amount::Amount;
use crate::error::ContractError;
use crate::ibc::Ics20Packet;
use crate::msg::{
    ChannelResponse, ExecuteMsg, InitMsg, ListChannelsResponse, PortResponse, QueryMsg, TransferMsg,
};
use crate::state::{Config, CHANNEL_INFO, CHANNEL_STATE, CONFIG};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cw20-ics20";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn init(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InitMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let cfg = Config {
        default_timeout: msg.default_timeout,
    };
    CONFIG.save(deps.storage, &cfg)?;
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Receive(msg) => execute_receive(deps, env, info, msg),
    }
}

pub fn execute_receive(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    wrapper: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let msg: TransferMsg = match wrapper.msg {
        Some(bin) => from_binary(&bin)?,
        None => return Err(ContractError::NoData {}),
    };
    let amount = Amount::Cw20(Cw20CoinHuman {
        address: info.sender.clone(),
        amount: wrapper.amount,
    });
    execute_transfer(deps, env, msg, amount, info.sender)
}

pub fn execute_transfer(
    deps: DepsMut,
    env: Env,
    msg: TransferMsg,
    amount: Amount,
    sender: HumanAddr,
) -> Result<Response, ContractError> {
    if amount.is_empty() {
        return Err(ContractError::NoFunds {});
    }
    // ensure the requested channel is registered
    // FIXME: add a .has method to map to make this faster
    if CHANNEL_INFO.may_load(deps.storage, &msg.channel)?.is_none() {
        return Err(ContractError::NoSuchChannel { id: msg.channel });
    }

    // delta from user is in seconds
    let timeout_delta = match msg.timeout {
        Some(t) => t,
        None => CONFIG.load(deps.storage)?.default_timeout,
    };
    // timeout is in nanoseconds
    let timeout = (env.block.time + timeout_delta) * 1_000_000_000;

    // build ics20 packet
    let packet = Ics20Packet {
        denom: amount.denom(),
        amount: amount.u64_amount()?,
        sender: sender.into(),
        receiver: msg.remote_address,
    };

    // prepare message
    let msg = IbcMsg::SendPacket {
        channel_id: msg.channel,
        data: to_binary(&packet)?,
        timeout_block: None,
        timeout_timestamp: Some(timeout),
    };

    // Note: we update local state when we get ack - do not count this transfer towards anything until acked

    // send response
    let res = Response {
        submessages: vec![],
        messages: vec![msg.into()],
        // TODO: more
        attributes: vec![attr("action", "transfer")],
        data: None,
    };
    Ok(res)
}

// fn send_tokens(
//     api: &dyn Api,
//     to: &HumanAddr,
//     balance: &GenericBalance,
// ) -> StdResult<Vec<CosmosMsg>> {
//     let native_balance = &balance.native;
//     let mut msgs: Vec<CosmosMsg> = if native_balance.is_empty() {
//         vec![]
//     } else {
//         vec![BankMsg::Send {
//             to_address: to.into(),
//             amount: native_balance.to_vec(),
//         }
//         .into()]
//     };
//
//     let cw20_balance = &balance.cw20;
//     let cw20_msgs: StdResult<Vec<_>> = cw20_balance
//         .iter()
//         .map(|c| {
//             let msg = Cw20HandleMsg::Transfer {
//                 recipient: to.into(),
//                 amount: c.amount,
//             };
//             let exec = WasmMsg::Execute {
//                 contract_addr: api.human_address(&c.address)?,
//                 msg: to_binary(&msg)?,
//                 send: vec![],
//             };
//             Ok(exec.into())
//         })
//         .collect();
//     msgs.append(&mut cw20_msgs?);
//     Ok(msgs)
// }

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Port {} => to_binary(&query_port(deps)?),
        QueryMsg::ListChannels {} => to_binary(&query_list(deps)?),
        QueryMsg::Channel { id } => to_binary(&query_channel(deps, id)?),
    }
}

fn query_port(deps: Deps) -> StdResult<PortResponse> {
    let query = IbcQuery::PortId {}.into();
    let PortIdResponse { port_id } = deps.querier.query(&query)?;
    Ok(PortResponse { port_id })
}

fn query_list(deps: Deps) -> StdResult<ListChannelsResponse> {
    let channels: StdResult<Vec<_>> = CHANNEL_INFO
        .range(deps.storage, None, None, Order::Ascending)
        .map(|r| r.map(|(_, v)| v))
        .collect();
    Ok(ListChannelsResponse {
        channels: channels?,
    })
}

fn query_channel(deps: Deps, id: String) -> StdResult<ChannelResponse> {
    let info = CHANNEL_INFO.load(deps.storage, &id)?;
    // this returns Vec<(outstanding, total)>
    let state: StdResult<Vec<_>> = CHANNEL_STATE
        .prefix(&id)
        .range(deps.storage, None, None, Order::Ascending)
        .map(|r| {
            let (k, v) = r?;
            let denom = String::from_utf8(k)?;
            let outstanding = Amount::from_parts(denom.clone(), v.outstanding);
            let total = Amount::from_parts(denom, v.total_sent);
            Ok((outstanding, total))
        })
        .collect();
    // we want (Vec<outstanding>, Vec<total>)
    let (balances, total_sent) = state?.into_iter().unzip();

    Ok(ChannelResponse {
        info,
        balances,
        total_sent,
    })
}

#[cfg(target_arch = "arm")]
mod tests {
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coin, coins, CanonicalAddr, CosmosMsg, StdError, Uint128};

    use crate::msg::HandleMsg::TopUp;

    use super::*;

    #[test]
    fn happy_path_native() {
        let mut deps = mock_dependencies(&[]);

        // init an empty contract
        let init_msg = InitMsg {};
        let info = mock_info(&HumanAddr::from("anyone"), &[]);
        let res = init(deps.as_mut(), mock_env(), info, init_msg).unwrap();
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
        let info = mock_info(&sender, &balance);
        let msg = HandleMsg::Create(create.clone());
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
        assert_eq!(attr("action", "create"), res.attributes[0]);

        // ensure the details is what we expect
        let details = query_details(deps.as_ref(), "foobar".to_string()).unwrap();
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
        let info = mock_info(&create.arbiter, &[]);
        let res = execute(deps.as_mut(), mock_env(), info, HandleMsg::Approve { id }).unwrap();
        assert_eq!(1, res.messages.len());
        assert_eq!(attr("action", "approve"), res.attributes[0]);
        assert_eq!(
            res.messages[0],
            CosmosMsg::Bank(BankMsg::Send {
                to_address: create.recipient,
                amount: balance,
            })
        );

        // second attempt fails (not found)
        let id = create.id.clone();
        let info = mock_info(&create.arbiter, &[]);
        let res = execute(deps.as_mut(), mock_env(), info, HandleMsg::Approve { id });
        match res.unwrap_err() {
            ContractError::Std(StdError::NotFound { .. }) => {}
            e => panic!("Expected NotFound, got {}", e),
        }
    }

    #[test]
    fn happy_path_cw20() {
        let mut deps = mock_dependencies(&[]);

        // init an empty contract
        let init_msg = InitMsg {};
        let info = mock_info(&HumanAddr::from("anyone"), &[]);
        let res = init(deps.as_mut(), mock_env(), info, init_msg).unwrap();
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
        let info = mock_info(&token_contract, &[]);
        let msg = HandleMsg::Receive(receive.clone());
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
        assert_eq!(attr("action", "create"), res.attributes[0]);

        // ensure the whitelist is what we expect
        let details = query_details(deps.as_ref(), "foobar".to_string()).unwrap();
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
        let info = mock_info(&create.arbiter, &[]);
        let res = execute(deps.as_mut(), mock_env(), info, HandleMsg::Approve { id }).unwrap();
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
        let info = mock_info(&create.arbiter, &[]);
        let res = execute(deps.as_mut(), mock_env(), info, HandleMsg::Approve { id });
        match res.unwrap_err() {
            ContractError::Std(StdError::NotFound { .. }) => {}
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
        let mut deps = mock_dependencies(&[]);

        // init an empty contract
        let init_msg = InitMsg {};
        let info = mock_info(&HumanAddr::from("anyone"), &[]);
        let res = init(deps.as_mut(), mock_env(), info, init_msg).unwrap();
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
        let info = mock_info(&sender, &balance);
        let msg = HandleMsg::Create(create.clone());
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
        assert_eq!(attr("action", "create"), res.attributes[0]);

        // top it up with 2 more native tokens
        let extra_native = vec![coin(250, "random"), coin(300, "stake")];
        let info = mock_info(&sender, &extra_native);
        let top_up = HandleMsg::TopUp {
            id: create.id.clone(),
        };
        let res = execute(deps.as_mut(), mock_env(), info, top_up).unwrap();
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
        let info = mock_info(&bar_token, &[]);
        let res = execute(deps.as_mut(), mock_env(), info, top_up).unwrap();
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
        let info = mock_info(&baz_token, &[]);
        let res = execute(deps.as_mut(), mock_env(), info, top_up);
        match res.unwrap_err() {
            ContractError::NotInWhitelist {} => {}
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
        let info = mock_info(&foo_token, &[]);
        let res = execute(deps.as_mut(), mock_env(), info, top_up).unwrap();
        assert_eq!(0, res.messages.len());
        assert_eq!(attr("action", "top_up"), res.attributes[0]);

        // approve it
        let id = create.id.clone();
        let info = mock_info(&create.arbiter, &[]);
        let res = execute(deps.as_mut(), mock_env(), info, HandleMsg::Approve { id }).unwrap();
        assert_eq!(attr("action", "approve"), res.attributes[0]);
        assert_eq!(3, res.messages.len());

        // first message releases all native coins
        assert_eq!(
            res.messages[0],
            CosmosMsg::Bank(BankMsg::Send {
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
