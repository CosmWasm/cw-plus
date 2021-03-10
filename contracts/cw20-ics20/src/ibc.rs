#![allow(clippy::field_reassign_with_default)] // see https://github.com/CosmWasm/cosmwasm/issues/685

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{
    attr, entry_point, from_binary, to_binary, BankMsg, Binary, CosmosMsg, DepsMut, Env, HumanAddr,
    IbcAcknowledgement, IbcBasicResponse, IbcChannel, IbcOrder, IbcPacket, IbcReceiveResponse,
    StdResult, Uint128, WasmMsg,
};

use crate::amount::Amount;
use crate::error::{ContractError, Never};
use crate::state::{ChannelInfo, CHANNEL_INFO, CHANNEL_STATE};
use cw20::Cw20HandleMsg;

pub const ICS20_VERSION: &str = "ics20-1";
pub const ICS20_ORDERING: IbcOrder = IbcOrder::Unordered;

/// The format for sending an ics20 packet.
/// Proto defined here: https://github.com/cosmos/cosmos-sdk/blob/v0.42.0/proto/ibc/applications/transfer/v1/transfer.proto#L11-L20
/// This is compatible with the JSON serialization
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug, Default)]
pub struct Ics20Packet {
    // the token denomination to be transferred
    pub denom: String,
    // TODO: is this encoded as a string? check real ics20 packets
    pub amount: u64,
    // the sender address
    pub sender: String,
    // the recipient address on the destination chain
    pub receiver: String,
}

/// This is a generic ICS acknowledgement format.
/// Proto defined here: https://github.com/cosmos/cosmos-sdk/blob/v0.42.0/proto/ibc/core/channel/v1/channel.proto#L141-L147
/// This is compatible with the JSON serialization
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Ics20Ack {
    Result(Binary),
    Error(String),
}

// create a serialized success message
fn ack_success() -> Binary {
    let res = Ics20Ack::Result(b"1".into());
    to_binary(&res).unwrap()
}

// create a serialized error message
fn ack_fail(err: String) -> Binary {
    let res = Ics20Ack::Error(err);
    to_binary(&res).unwrap()
}

#[cfg_attr(not(feature = "library"), entry_point)]
/// enforces ordering and versioning constraints
pub fn ibc_channel_open(
    _deps: DepsMut,
    _env: Env,
    channel: IbcChannel,
) -> Result<(), ContractError> {
    enforce_order_and_version(&channel)?;
    Ok(())
}

#[cfg_attr(not(feature = "library"), entry_point)]
/// record the channel in CHANNEL_INFO
pub fn ibc_channel_connect(
    deps: DepsMut,
    _env: Env,
    channel: IbcChannel,
) -> Result<IbcBasicResponse, ContractError> {
    // we need to check the counter party version in try and ack (sometimes here)
    enforce_order_and_version(&channel)?;

    let info = ChannelInfo {
        id: channel.endpoint.channel_id,
        counterparty_endpoint: channel.counterparty_endpoint,
        connection_id: channel.connection_id,
    };
    CHANNEL_INFO.save(deps.storage, &info.id, &info)?;

    Ok(IbcBasicResponse::default())
}

fn enforce_order_and_version(channel: &IbcChannel) -> Result<(), ContractError> {
    if channel.version != ICS20_VERSION {
        return Err(ContractError::InvalidIbcVersion {
            version: channel.version.clone(),
        });
    }
    if let Some(version) = &channel.counterparty_version {
        if version != ICS20_VERSION {
            return Err(ContractError::InvalidIbcVersion {
                version: version.clone(),
            });
        }
    }
    if channel.order != ICS20_ORDERING {
        return Err(ContractError::OnlyOrderedChannel {});
    }
    Ok(())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn ibc_channel_close(
    _deps: DepsMut,
    _env: Env,
    _channel: IbcChannel,
) -> Result<IbcBasicResponse, ContractError> {
    // TODO: what to do here?
    // we will have locked funds that need to be returned somehow
    unimplemented!();
}

#[cfg_attr(not(feature = "library"), entry_point)]
/// Check to see if we have any balance here
/// We should not return an error if possible, but rather an acknowledgement of failure
pub fn ibc_packet_receive(
    deps: DepsMut,
    _env: Env,
    packet: IbcPacket,
) -> Result<IbcReceiveResponse, Never> {
    let res = match do_ibc_packet_receive(deps, packet) {
        Ok(msg) => {
            // build attributes first so we don't have to clone msg below
            // similar event messages like ibctransfer module
            let attributes = vec![
                attr("action", "receive"),
                attr("sender", &msg.sender),
                attr("receiver", &msg.receiver),
                attr("denom", &msg.denom),
                attr("amount", &msg.amount),
                attr("success", "true"),
            ];
            let to_send = Amount::from_parts(msg.denom, msg.amount.into());
            let msg = send_amount(to_send, HumanAddr::from(msg.receiver));
            IbcReceiveResponse {
                acknowledgement: ack_success(),
                messages: vec![msg],
                attributes,
            }
        }
        Err(err) => IbcReceiveResponse {
            acknowledgement: ack_fail(err.to_string()),
            messages: vec![],
            attributes: vec![
                attr("action", "receive"),
                attr("success", "false"),
                attr("error", err),
            ],
        },
    };

    // if we have funds, now send the tokens to the requested recipient
    Ok(res)
}

// this does the work of ibc_packet_receive, we wrap it to turn errors into acknowledgements
fn do_ibc_packet_receive(deps: DepsMut, packet: IbcPacket) -> Result<Ics20Packet, ContractError> {
    let msg: Ics20Packet = from_binary(&packet.data)?;
    let channel = packet.dest.channel_id;
    let denom = msg.denom.clone();
    let amount = Uint128::from(msg.amount);
    CHANNEL_STATE.update(
        deps.storage,
        (&channel, &denom),
        |orig| -> Result<_, ContractError> {
            // this will return error if we don't have the funds there to cover the request (or no denom registered)
            let mut cur = orig.ok_or(ContractError::InsufficientFunds {})?;
            cur.outstanding =
                (cur.outstanding - amount).or(Err(ContractError::InsufficientFunds {}))?;
            Ok(cur)
        },
    )?;
    Ok(msg)
}

#[cfg_attr(not(feature = "library"), entry_point)]
/// check if success or failure and update balance, or return funds
pub fn ibc_packet_ack(
    deps: DepsMut,
    _env: Env,
    ack: IbcAcknowledgement,
) -> Result<IbcBasicResponse, ContractError> {
    // TODO: trap error like in receive?
    let msg: Ics20Ack = from_binary(&ack.acknowledgement)?;
    match msg {
        Ics20Ack::Result(_) => on_packet_success(deps, ack.original_packet),
        Ics20Ack::Error(err) => on_packet_failure(deps, ack.original_packet, err),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
/// return fund to original sender (same as failure in ibc_packet_ack)
pub fn ibc_packet_timeout(
    deps: DepsMut,
    _env: Env,
    packet: IbcPacket,
) -> Result<IbcBasicResponse, ContractError> {
    // TODO: trap error like in receive?
    on_packet_failure(deps, packet, "timeout".to_string())
}

// update the balance stored on this (channel, denom) index
fn on_packet_success(deps: DepsMut, packet: IbcPacket) -> Result<IbcBasicResponse, ContractError> {
    let msg: Ics20Packet = from_binary(&packet.data)?;
    // similar event messages like ibctransfer module
    let attributes = vec![
        attr("action", "acknowledge"),
        attr("sender", &msg.sender),
        attr("receiver", &msg.receiver),
        attr("denom", &msg.denom),
        attr("amount", &msg.amount),
        attr("success", "true"),
    ];

    let channel = packet.src.channel_id;
    let denom = msg.denom;
    let amount = Uint128::from(msg.amount);
    CHANNEL_STATE.update(deps.storage, (&channel, &denom), |orig| -> StdResult<_> {
        let mut state = orig.unwrap_or_default();
        state.outstanding += amount;
        state.total_sent += amount;
        Ok(state)
    })?;

    Ok(IbcBasicResponse {
        messages: vec![],
        attributes,
    })
}

// return the tokens to sender
fn on_packet_failure(
    _deps: DepsMut,
    packet: IbcPacket,
    err: String,
) -> Result<IbcBasicResponse, ContractError> {
    let msg: Ics20Packet = from_binary(&packet.data)?;
    // similar event messages like ibctransfer module
    let attributes = vec![
        attr("action", "acknowledge"),
        attr("sender", &msg.sender),
        attr("receiver", &msg.receiver),
        attr("denom", &msg.denom),
        attr("amount", &msg.amount),
        attr("success", "false"),
        attr("error", err),
    ];

    let amount = Amount::from_parts(msg.denom, msg.amount.into());
    let msg = send_amount(amount, HumanAddr::from(msg.sender));
    let res = IbcBasicResponse {
        messages: vec![msg],
        attributes,
    };
    Ok(res)
}

fn send_amount(amount: Amount, recipient: HumanAddr) -> CosmosMsg {
    match amount {
        Amount::Native(coin) => BankMsg::Send {
            to_address: recipient,
            amount: vec![coin],
        }
        .into(),
        Amount::Cw20(coin) => {
            let msg = Cw20HandleMsg::Transfer {
                recipient,
                amount: coin.amount,
            };
            let exec = WasmMsg::Execute {
                contract_addr: coin.address,
                msg: to_binary(&msg).unwrap(),
                send: vec![],
            };
            exec.into()
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test_helpers::*;

    use crate::contract::query_channel;
    use cosmwasm_std::testing::mock_env;
    use cosmwasm_std::{coins, to_vec, IbcEndpoint};

    fn cw20_payment(amount: u128, address: &str, recipient: &str) -> CosmosMsg {
        let msg = Cw20HandleMsg::Transfer {
            recipient: recipient.into(),
            amount: Uint128(amount),
        };
        let exec = WasmMsg::Execute {
            contract_addr: address.into(),
            msg: to_binary(&msg).unwrap(),
            send: vec![],
        };
        exec.into()
    }

    #[allow(dead_code)]
    fn native_payment(amount: u128, denom: &str, recipient: &str) -> CosmosMsg {
        BankMsg::Send {
            to_address: recipient.into(),
            amount: coins(amount, denom),
        }
        .into()
    }

    fn mock_sent_packet(my_channel: &str, amount: u64, denom: &str, sender: &str) -> IbcPacket {
        let data = Ics20Packet {
            denom: denom.into(),
            amount,
            sender: sender.to_string(),
            receiver: "remote-rcpt".to_string(),
        };
        IbcPacket {
            data: to_binary(&data).unwrap(),
            src: IbcEndpoint {
                port_id: CONTRACT_PORT.to_string(),
                channel_id: my_channel.to_string(),
            },
            dest: IbcEndpoint {
                port_id: REMOTE_PORT.to_string(),
                channel_id: "channel-1234".to_string(),
            },
            sequence: 2,
            timeout_block: None,
            timeout_timestamp: Some(1665321069000000000u64),
        }
    }
    fn mock_receive_packet(
        my_channel: &str,
        amount: u64,
        denom: &str,
        receiver: &str,
    ) -> IbcPacket {
        let data = Ics20Packet {
            denom: denom.into(),
            amount,
            sender: "remote-sender".to_string(),
            receiver: receiver.to_string(),
        };
        IbcPacket {
            data: to_binary(&data).unwrap(),
            src: IbcEndpoint {
                port_id: REMOTE_PORT.to_string(),
                channel_id: "channel-1234".to_string(),
            },
            dest: IbcEndpoint {
                port_id: CONTRACT_PORT.to_string(),
                channel_id: my_channel.to_string(),
            },
            sequence: 3,
            timeout_block: None,
            timeout_timestamp: Some(1665321069000000000u64),
        }
    }

    #[test]
    fn check_ack_json() {
        let success = Ics20Ack::Result(b"1".into());
        let fail = Ics20Ack::Error("bad coin".into());

        let success_json = String::from_utf8(to_vec(&success).unwrap()).unwrap();
        assert_eq!(r#"{"result":"MQ=="}"#, success_json.as_str());

        let fail_json = String::from_utf8(to_vec(&fail).unwrap()).unwrap();
        assert_eq!(r#"{"error":"bad coin"}"#, fail_json.as_str());
    }

    #[test]
    fn send_receive_cw20() {
        let send_channel = "channel-9";
        let mut deps = setup(&["channel-1", "channel-7", send_channel]);

        let cw20_addr = "token-addr";
        let cw20_denom = "cw20:token-addr";

        // prepare some mock packets
        let sent_packet = mock_sent_packet(send_channel, 987654321, cw20_denom, "local-sender");
        let recv_packet = mock_receive_packet(send_channel, 876543210, cw20_denom, "local-rcpt");
        let recv_high_packet =
            mock_receive_packet(send_channel, 1876543210, cw20_denom, "local-rcpt");

        // cannot receive this denom yet
        let res = ibc_packet_receive(deps.as_mut(), mock_env(), recv_packet.clone()).unwrap();
        assert!(res.messages.is_empty());
        let ack: Ics20Ack = from_binary(&res.acknowledgement).unwrap();
        let no_funds = Ics20Ack::Error(ContractError::InsufficientFunds {}.to_string());
        assert_eq!(ack, no_funds);

        // we get a success cache (ack) for a send
        let ack = IbcAcknowledgement {
            acknowledgement: ack_success(),
            original_packet: sent_packet.clone(),
        };
        let res = ibc_packet_ack(deps.as_mut(), mock_env(), ack).unwrap();
        assert_eq!(0, res.messages.len());

        // query channel state|_|
        let state = query_channel(deps.as_ref(), send_channel.to_string()).unwrap();
        assert_eq!(state.balances, vec![Amount::cw20(987654321, cw20_addr)]);
        assert_eq!(state.total_sent, vec![Amount::cw20(987654321, cw20_addr)]);

        // cannot receive more than we sent
        let res = ibc_packet_receive(deps.as_mut(), mock_env(), recv_high_packet).unwrap();
        assert!(res.messages.is_empty());
        let ack: Ics20Ack = from_binary(&res.acknowledgement).unwrap();
        assert_eq!(ack, no_funds);

        // we can receive less than we sent
        let res = ibc_packet_receive(deps.as_mut(), mock_env(), recv_packet).unwrap();
        assert_eq!(1, res.messages.len());
        assert_eq!(
            cw20_payment(876543210, cw20_addr, "local-rcpt"),
            res.messages[0]
        );
        let ack: Ics20Ack = from_binary(&res.acknowledgement).unwrap();
        matches!(ack, Ics20Ack::Result(_));

        // query channel state
        let state = query_channel(deps.as_ref(), send_channel.to_string()).unwrap();
        assert_eq!(state.balances, vec![Amount::cw20(111111111, cw20_addr)]);
        assert_eq!(state.total_sent, vec![Amount::cw20(987654321, cw20_addr)]);
    }
}