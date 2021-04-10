#![cfg(test)]

use crate::contract::instantiate;
use crate::ibc::{ibc_channel_connect, ibc_channel_open, ICS20_ORDERING, ICS20_VERSION};
use crate::state::ChannelInfo;

use cosmwasm_std::testing::{
    mock_dependencies, mock_env, mock_info, MockApi, MockQuerier, MockStorage,
};
use cosmwasm_std::{DepsMut, IbcChannel, IbcEndpoint, OwnedDeps};

use crate::msg::InitMsg;

pub const DEFAULT_TIMEOUT: u64 = 3600; // 1 hour,
pub const CONTRACT_PORT: &str = "ibc:wasm1234567890abcdef";
pub const REMOTE_PORT: &str = "transfer";
pub const CONNECTION_ID: &str = "connection-2";

pub fn mock_channel(channel_id: &str) -> IbcChannel {
    IbcChannel {
        endpoint: IbcEndpoint {
            port_id: CONTRACT_PORT.into(),
            channel_id: channel_id.into(),
        },
        counterparty_endpoint: IbcEndpoint {
            port_id: REMOTE_PORT.into(),
            channel_id: format!("{}5", channel_id),
        },
        order: ICS20_ORDERING,
        version: ICS20_VERSION.into(),
        counterparty_version: None,
        connection_id: CONNECTION_ID.into(),
    }
}

pub fn mock_channel_info(channel_id: &str) -> ChannelInfo {
    ChannelInfo {
        id: channel_id.to_string(),
        counterparty_endpoint: IbcEndpoint {
            port_id: REMOTE_PORT.into(),
            channel_id: format!("{}5", channel_id),
        },
        connection_id: CONNECTION_ID.into(),
    }
}

// we simulate instantiate and ack here
pub fn add_channel(mut deps: DepsMut, channel_id: &str) {
    let mut channel = mock_channel(channel_id);
    ibc_channel_open(deps.branch(), mock_env(), channel.clone()).unwrap();
    channel.counterparty_version = Some(ICS20_VERSION.into());
    ibc_channel_connect(deps.branch(), mock_env(), channel).unwrap();
}

pub fn setup(channels: &[&str]) -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
    let mut deps = mock_dependencies(&[]);

    // instantiate an empty contract
    let instantiate_msg = InitMsg {
        default_timeout: DEFAULT_TIMEOUT,
    };
    let info = mock_info(&String::from("anyone"), &[]);
    let res = instantiate(deps.as_mut(), mock_env(), info, instantiate_msg).unwrap();
    assert_eq!(0, res.messages.len());

    for channel in channels {
        add_channel(deps.as_mut(), channel);
    }
    deps
}
