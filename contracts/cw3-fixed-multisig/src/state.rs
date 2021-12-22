use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};
use utils::{Duration, Threshold};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Config {
    pub threshold: Threshold,
    pub total_weight: u64,
    pub max_voting_period: Duration,
}

// unique items
pub const CONFIG: Item<Config> = Item::new("config");
pub const PROPOSAL_COUNT: Item<u64> = Item::new("proposal_count");

// multiple-item maps
pub const VOTERS: Map<&Addr, u64> = Map::new("voters");
