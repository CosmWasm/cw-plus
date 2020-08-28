use cosmwasm_std::{StdResult, Storage};

/// this takes a v0.1.x store and converts it to a v0.2.x format
pub fn migrate_v01_to_v02<S: Storage>(storage: &mut S) -> StdResult<()> {
    // TODO
    Ok(())
}
