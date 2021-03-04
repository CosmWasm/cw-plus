use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use cosmwasm_std::{
    attr, CosmosMsg, Deps, DepsMut, HumanAddr, MessageInfo, Response, StdError, StdResult, Storage,
};
use cw_storage_plus::Item;

use crate::admin::{Admin, AdminError};

// this is copied from cw4
// TODO: pull into cw0 as common dep
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct HooksResponse {
    pub hooks: Vec<HumanAddr>,
}

#[derive(Error, Debug, PartialEq)]
pub enum HookError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Admin(#[from] AdminError),

    #[error("Given address already registered as a hook")]
    HookAlreadyRegistered {},

    #[error("Given address not registered as a hook")]
    HookNotRegistered {},
}

// store all hook addresses in one item. We cannot have many of them before the contract becomes unusable anyway.
pub struct Hooks<'a>(Item<'a, Vec<HumanAddr>>);

impl<'a> Hooks<'a> {
    pub const fn new(storage_key: &'a str) -> Self {
        Hooks(Item::new(storage_key))
    }

    pub fn add_hook(&self, storage: &mut dyn Storage, addr: HumanAddr) -> Result<(), HookError> {
        let mut hooks = self.0.may_load(storage)?.unwrap_or_default();
        if !hooks.iter().any(|h| h == &addr) {
            hooks.push(addr);
        } else {
            return Err(HookError::HookAlreadyRegistered {});
        }
        Ok(self.0.save(storage, &hooks)?)
    }

    pub fn remove_hook(&self, storage: &mut dyn Storage, addr: HumanAddr) -> Result<(), HookError> {
        let mut hooks = self.0.load(storage)?;
        if let Some(p) = hooks.iter().position(|x| x == &addr) {
            hooks.remove(p);
        } else {
            return Err(HookError::HookNotRegistered {});
        }
        Ok(self.0.save(storage, &hooks)?)
    }

    pub fn prepare_hooks<F: Fn(HumanAddr) -> StdResult<CosmosMsg>>(
        &self,
        storage: &dyn Storage,
        prep: F,
    ) -> StdResult<Vec<CosmosMsg>> {
        self.0
            .may_load(storage)?
            .unwrap_or_default()
            .into_iter()
            .map(prep)
            .collect()
    }

    pub fn handle_add_hook(
        &self,
        admin: &Admin,
        deps: DepsMut,
        info: MessageInfo,
        addr: HumanAddr,
    ) -> Result<Response, HookError> {
        admin.assert_admin(deps.as_ref(), &info.sender)?;
        self.add_hook(deps.storage, addr.clone())?;

        let attributes = vec![
            attr("action", "add_hook"),
            attr("hook", addr),
            attr("sender", info.sender),
        ];
        Ok(Response {
            submessages: vec![],
            messages: vec![],
            attributes,
            data: None,
        })
    }

    pub fn handle_remove_hook(
        &self,
        admin: &Admin,
        deps: DepsMut,
        info: MessageInfo,
        addr: HumanAddr,
    ) -> Result<Response, HookError> {
        admin.assert_admin(deps.as_ref(), &info.sender)?;
        self.remove_hook(deps.storage, addr.clone())?;

        let attributes = vec![
            attr("action", "remove_hook"),
            attr("hook", addr),
            attr("sender", info.sender),
        ];
        Ok(Response {
            submessages: vec![],
            messages: vec![],
            attributes,
            data: None,
        })
    }

    pub fn query_hooks(&self, deps: Deps) -> StdResult<HooksResponse> {
        let hooks = self.0.may_load(deps.storage)?.unwrap_or_default();
        Ok(HooksResponse { hooks })
    }
}

// TODO: add test coverage
