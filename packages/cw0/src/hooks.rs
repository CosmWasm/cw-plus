use cosmwasm_std::{CosmosMsg, HumanAddr, StdError, StdResult, Storage};

use cw_storage_plus::Item;

// store all hook addresses in one item. We cannot have many of them before the contract becomes unusable anyway.
pub const HOOKS: Item<Vec<HumanAddr>> = Item::new("hooks");

// Returning a custom error here would be a headache - especially for the importing contract.
// rather I just define the error messages here and use StdError::GenericErr
pub const HOOK_ALREADY_REGISTERED: &str = "Given address already registered as a hook";
pub const HOOK_NOT_REGISTERED: &str = "Given address not registered as a hook";

pub fn add_hook(storage: &mut dyn Storage, addr: HumanAddr) -> StdResult<()> {
    let mut hooks = HOOKS.may_load(storage)?.unwrap_or_default();
    if !hooks.iter().any(|h| h == &addr) {
        hooks.push(addr);
    } else {
        return Err(StdError::generic_err(HOOK_ALREADY_REGISTERED));
    }
    HOOKS.save(storage, &hooks)
}

pub fn remove_hook(storage: &mut dyn Storage, addr: HumanAddr) -> StdResult<()> {
    let mut hooks = HOOKS.load(storage)?;
    if let Some(p) = hooks.iter().position(|x| x == &addr) {
        hooks.remove(p);
    } else {
        return Err(StdError::generic_err(HOOK_NOT_REGISTERED));
    }
    HOOKS.save(storage, &hooks)
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
