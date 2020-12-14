use cosmwasm_std::CanonicalAddr;
use cw4::TOTAL_KEY;
use cw_storage_plus::{Item, SnapshotMap, Strategy};

pub const ADMIN: Item<Option<CanonicalAddr>> = Item::new("admin");
pub const TOTAL: Item<u64> = Item::new(TOTAL_KEY);

pub const MEMBERS: SnapshotMap<&[u8], u64> = SnapshotMap::new(
    cw4::MEMBERS_KEY,
    cw4::MEMBERS_CHECKPOINTS,
    cw4::MEMBERS_CHANGELOG,
    Strategy::EveryBlock,
);
