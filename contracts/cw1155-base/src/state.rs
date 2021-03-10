use cosmwasm_std::{CanonicalAddr, Uint128};
use cw_storage_plus::{Item, Map};

pub const MINTER: Item<CanonicalAddr> = Item::new("minter");
pub const TOKENS: Map<(&str, &[u8]), Uint128> = Map::new("tokens");
pub const APPROVES: Map<&[u8], CanonicalAddr> = Map::new("approves");
