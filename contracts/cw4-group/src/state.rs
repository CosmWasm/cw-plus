use cosmwasm_std::CanonicalAddr;
use cw_storage_plus::{Item, Map, Path};

/// TOTAL_KEY is meant for raw queries
pub const TOTAL_KEY: &[u8] = b"total";
const MEMBERS_KEY: &[u8] = b"members";

pub const ADMIN: Item<Option<CanonicalAddr>> = Item::new(b"admin");
pub const TOTAL: Item<u64> = Item::new(TOTAL_KEY);
pub const MEMBERS: Map<&[u8], u64> = Map::new(MEMBERS_KEY);

/// member_path is meant for raw queries
pub fn member_path(address: &[u8]) -> Path<u64> {
    Path::new(MEMBERS_KEY, &[address])
}
