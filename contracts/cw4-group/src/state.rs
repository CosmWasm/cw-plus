use cosmwasm_std::{CanonicalAddr, HumanAddr};
use cw4::TOTAL_KEY;
use cw_storage_plus::{snapshot_names, Item, SnapshotMap, SnapshotNamespaces, Strategy};

pub const ADMIN: Item<Option<CanonicalAddr>> = Item::new(b"admin");
pub const TOTAL: Item<u64> = Item::new(TOTAL_KEY);

// Note: this must be same as cw4::MEMBERS_KEY but macro needs literal, not const
pub const MEMBERS: SnapshotMap<&[u8], u64> =
    SnapshotMap::new(snapshot_names!("members"), Strategy::EveryBlock);

// store all hook addresses in one item. We cannot have many of them before the contract
// becomes unusable
pub const HOOKS: Item<Vec<HumanAddr>> = Item::new(b"hooks");
