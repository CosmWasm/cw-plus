use cosmwasm_std::{
    CanonicalAddr, Deps, DepsMut, HandleResponse, HumanAddr, MessageInfo, StdError, StdResult,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::ops::Deref;
use thiserror::Error;

use cw0::maybe_canonical;
use cw_storage_plus::Item;

// state/logic
pub struct Admin<'a>(Item<'a, Option<CanonicalAddr>>);

// allow easy access to the basic Item operations if desired
// TODO: reconsider if we need this here, maybe only for maps?
impl<'a> Deref for Admin<'a> {
    type Target = Item<'a, Option<CanonicalAddr>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// this is the core business logic we expose
impl<'a> Admin<'a> {
    pub const fn new(namespace: &'a str) -> Self {
        Admin(Item::new(namespace))
    }

    pub fn set(&self, deps: DepsMut, admin: Option<HumanAddr>) -> StdResult<()> {
        let admin_raw = maybe_canonical(deps.api, admin)?;
        self.save(deps.storage, &admin_raw)
    }

    pub fn get(&self, deps: Deps) -> StdResult<Option<HumanAddr>> {
        let canon = self.load(deps.storage)?;
        canon.map(|c| deps.api.human_address(&c)).transpose()
    }

    /// Returns Ok(true) if this is an admin, Ok(false) if not and an Error if
    /// we hit an error with Api or Storage usage
    pub fn is_admin(&self, deps: Deps, caller: &HumanAddr) -> StdResult<bool> {
        match self.load(deps.storage)? {
            Some(owner) => {
                let caller_raw = deps.api.canonical_address(caller)?;
                Ok(caller_raw == owner)
            }
            None => Ok(false),
        }
    }

    /// Like is_admin but returns AdminError::NotAdmin if not admin.
    /// Helper for a nice one-line auth check.
    pub fn assert_admin(&self, deps: Deps, caller: &HumanAddr) -> Result<(), AdminError> {
        if !self.is_admin(deps, caller)? {
            return Err(AdminError::NotAdmin {});
        } else {
            Ok(())
        }
    }
}

// messages
// TODO: should all the definitions end up in cw0, so eg. cw4 can import them as well as this module?
// Or should the cwX specs not define these types at all, and just require the base contract to mix this in?

/// This should be embedded in parent `HandleMsg` as `UpdateAdmin(UpdateAdminHandleMsg)`
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct UpdateAdminHandleMsg {
    admin: Option<HumanAddr>,
}

/// This should be embedded in parent `QueryMsg` as `Admin(AdminQueryMsg)`
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct AdminQueryMsg {}

/// Returned from AdminQueryMsg
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct AdminResponse {
    pub admin: Option<HumanAddr>,
}

// errors

#[derive(Error, Debug)]
pub enum AdminError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Caller is not admin")]
    NotAdmin {},
}

// handlers: maybe they don't even make sense, as they are so simple???

pub fn handle_update_admin(
    // we need to pass in the controller from the contract, so we know which key it uses
    // better naming here?
    controller: &Admin,
    deps: DepsMut,
    info: MessageInfo,
    new_admin: Option<HumanAddr>,
) -> Result<HandleResponse, AdminError> {
    controller.assert_admin(deps.as_ref(), &info.sender)?;
    controller.set(deps, new_admin)?;
    // TODO: add some common log attributes here
    Ok(HandleResponse::default())
}

pub fn query_admin(controller: &Admin, deps: Deps) -> StdResult<AdminResponse> {
    let admin = controller.get(deps)?;
    Ok(AdminResponse { admin })
}
