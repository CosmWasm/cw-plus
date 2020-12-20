use cosmwasm_std::{CosmosMsg, HumanAddr, StdError, StdResult, Storage};
use thiserror::Error;

use cw_storage_plus::Item;
use std::ops::Deref;

#[derive(Error, Debug, PartialEq)]
pub enum HookError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Given address already registered as a hook")]
    HookAlreadyRegistered {},

    #[error("Given address not registered as a hook")]
    HookNotRegistered {},
}

// store all hook addresses in one item. We cannot have many of them before the contract becomes unusable anyway.
pub struct Hooks<'a>(Item<'a, Vec<HumanAddr>>);

// allow easy access to the basic Item operations if desired
// TODO: reconsider if we need this here, maybe only for maps?
impl<'a> Deref for Hooks<'a> {
    type Target = Item<'a, Vec<HumanAddr>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> Hooks<'a> {
    pub fn new(storage_key: &'a str) -> Self {
        Hooks(Item::new(storage_key))
    }

    pub fn add_hook(&self, storage: &mut dyn Storage, addr: HumanAddr) -> Result<(), HookError> {
        let mut hooks = self.may_load(storage)?.unwrap_or_default();
        if !hooks.iter().any(|h| h == &addr) {
            hooks.push(addr);
        } else {
            return Err(HookError::HookAlreadyRegistered {});
        }
        Ok(self.save(storage, &hooks)?)
    }

    pub fn remove_hook(&self, storage: &mut dyn Storage, addr: HumanAddr) -> Result<(), HookError> {
        let mut hooks = self.load(storage)?;
        if let Some(p) = hooks.iter().position(|x| x == &addr) {
            hooks.remove(p);
        } else {
            return Err(HookError::HookNotRegistered {});
        }
        Ok(self.save(storage, &hooks)?)
    }

    pub fn prepare_hooks<F: Fn(HumanAddr) -> StdResult<CosmosMsg>>(
        &self,
        storage: &dyn Storage,
        prep: F,
    ) -> StdResult<Vec<CosmosMsg>> {
        self.may_load(storage)?
            .unwrap_or_default()
            .into_iter()
            .map(prep)
            .collect()
    }
}
