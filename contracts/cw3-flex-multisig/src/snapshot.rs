use cosmwasm_std::{CanonicalAddr, Deps, DepsMut, HumanAddr, Order, StdResult, Storage};
use cw3::VoterInfo;
use cw4::{Cw4Contract, MemberDiff};
use cw_storage_plus::{Bound, Map, U64Key};

/// SNAPSHOTS pk: (HUmanAddr, height) -> get the weight
/// Use VoterInfo, so None (no data) is different than VoterInfo{weight: None} (record it was removed)
pub const SNAPSHOTS: Map<(&[u8], U64Key), VoterInfo> = Map::new(b"snapshots");

/// load the weight from the snapshot - that is, first change >= height,
/// or query the current contract state otherwise
pub fn snapshoted_weight(
    deps: Deps,
    addr: &HumanAddr,
    height: u64,
    group: &Cw4Contract,
) -> StdResult<Option<u64>> {
    let raw_addr = deps.api.canonical_address(addr)?;

    let snapshot = load_snapshot(deps.storage, &raw_addr, height)?;
    match snapshot {
        // use snapshot if available
        Some(info) => Ok(info.weight),
        // otherwise load from the group
        None => group.is_member(&deps.querier, &raw_addr),
    }
}

/// saves this diff only if no updates have been saved since the latest snapshot
pub fn snapshot_diff(
    deps: DepsMut,
    diff: MemberDiff,
    current_height: u64,
    latest_snapshot_height: u64,
) -> StdResult<()> {
    let raw_addr = deps.api.canonical_address(&diff.addr)?;
    match load_snapshot(deps.storage, &raw_addr, latest_snapshot_height)? {
        Some(_) => Ok(()),
        None => SNAPSHOTS.save(
            deps.storage,
            (&raw_addr, current_height.into()),
            &VoterInfo {
                weight: diff.old_weight,
            },
        ),
    }
}

/// this will look for the first snapshot of the given address >= given height
/// if none, there is no snapshot since that time, and the group query will give the current status
fn load_snapshot(
    storage: &dyn Storage,
    addr: &CanonicalAddr,
    height: u64,
) -> StdResult<Option<VoterInfo>> {
    let start = Bound::inclusive(U64Key::new(height));
    let first = SNAPSHOTS
        .prefix(&addr)
        .range(storage, Some(start), None, Order::Ascending)
        .next();
    match first {
        None => Ok(None),
        Some(r) => r.map(|(_, v)| Some(v)),
    }
}
