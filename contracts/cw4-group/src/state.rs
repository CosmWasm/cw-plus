use cosmwasm_std::{CanonicalAddr, HumanAddr};
use cw4::{MEMBERS_KEY, TOTAL_KEY};
use cw_storage_plus::{Item, Map};

pub const ADMIN: Item<Option<CanonicalAddr>> = Item::new(b"admin");
pub const TOTAL: Item<u64> = Item::new(TOTAL_KEY);
pub const MEMBERS: Map<&[u8], u64> = Map::new(MEMBERS_KEY);
// store all hook addresses in one item. We cannot have many of them before the contract
// becomes unusable
pub const HOOKS: Item<Vec<HumanAddr>> = Item::new(b"hooks");
