use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::Uint128;
use cw_storage_plus::{Item, Map};

use crate::msg::ChannelInfo;

pub const CONFIG: Item<Config> = Item::new("ics20_config");

// static info on one channel that doesn't change
pub const CHANNEL_INFO: Map<&str, ChannelInfo> = Map::new("channel_info");

// indexed by (channel_id, denom) maintaining the balance of the channel in that currency
pub const CHANNEL_STATE: Map<(&str, &str), Balance> = Map::new("channel_balance");

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug, Default)]
pub struct Balance {
    pub outstanding: Uint128,
    pub total_send: Uint128,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug, Default)]
pub struct Config {
    pub default_timeout: u64,
}
