use cosmwasm_std::{CosmosMsg, HumanAddr, StdError, StdResult, Storage};
use thiserror::Error;

use cw_storage_plus::Item;

// store all hook addresses in one item. We cannot have many of them before the contract becomes unusable anyway.
pub const HOOKS: Item<Vec<HumanAddr>> = Item::new("hooks");

#[derive(Error, Debug)]
pub enum HookError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Given address already registered as a hook")]
    HookAlreadyRegistered {},

    #[error("Given address not registered as a hook")]
    HookNotRegistered {},
}

pub fn add_hook(storage: &mut dyn Storage, addr: HumanAddr) -> Result<(), HookError> {
    let mut hooks = HOOKS.may_load(storage)?.unwrap_or_default();
    if !hooks.iter().any(|h| h == &addr) {
        hooks.push(addr);
    } else {
        return Err(HookError::HookAlreadyRegistered {});
    }
    Ok(HOOKS.save(storage, &hooks)?)
}

pub fn remove_hook(storage: &mut dyn Storage, addr: HumanAddr) -> Result<(), HookError> {
    let mut hooks = HOOKS.load(storage)?;
    if let Some(p) = hooks.iter().position(|x| x == &addr) {
        hooks.remove(p);
    } else {
        return Err(HookError::HookNotRegistered {});
    }
    Ok(HOOKS.save(storage, &hooks)?)
}

pub fn prepare_hooks<F: Fn(HumanAddr) -> StdResult<CosmosMsg>>(
    storage: &dyn Storage,
    prep: F,
) -> StdResult<Vec<CosmosMsg>> {
    HOOKS
        .may_load(storage)?
        .unwrap_or_default()
        .into_iter()
        .map(prep)
        .collect()
}
