use cosmwasm_std::CanonicalAddr;
use cw_storage_plus::{Item, Map};

pub const ADMIN: Item<Option<CanonicalAddr>> = Item::new(b"admin");
pub const TOTAL: Item<u64> = Item::new(b"total");
pub const MEMBERS: Map<&[u8], u64> = Map::new(b"members");
