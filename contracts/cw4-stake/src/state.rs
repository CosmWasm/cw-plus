use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{CanonicalAddr, Uint128};
use cw0::Duration;
use cw4::TOTAL_KEY;
use cw_storage_plus::{snapshot_names, Item, Map, SnapshotMap, SnapshotNamespaces, Strategy};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Config {
    /// denom of the token to stake
    pub denom: String,
    pub tokens_per_weight: Uint128,
    pub min_bond: Uint128,
    pub unbonding_period: Duration,
}

pub const ADMIN: Item<Option<CanonicalAddr>> = Item::new(b"admin");
pub const CONFIG: Item<Config> = Item::new(b"config");
pub const TOTAL: Item<u64> = Item::new(TOTAL_KEY);

// Note: this must be same as cw4::MEMBERS_KEY but macro needs literal, not const
pub const MEMBERS: SnapshotMap<&[u8], u64> =
    SnapshotMap::new(snapshot_names!("members"), Strategy::EveryBlock);

pub const STAKE: Map<&[u8], Uint128> = Map::new(b"stake");
