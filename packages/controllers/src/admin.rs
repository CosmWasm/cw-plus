use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use cosmwasm_std::{
    attr, CanonicalAddr, Deps, DepsMut, HandleResponse, HumanAddr, MessageInfo, StdError, StdResult,
};
use cw0::maybe_canonical;
use cw_storage_plus::Item;

// TODO: should the return values end up in cw0, so eg. cw4 can import them as well as this module?
/// Returned from Admin.query_admin()
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct AdminResponse {
    pub admin: Option<HumanAddr>,
}

/// Errors returned from Admin
#[derive(Error, Debug, PartialEq)]
pub enum AdminError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Caller is not admin")]
    NotAdmin {},
}

// state/logic
pub struct Admin<'a>(Item<'a, Option<CanonicalAddr>>);

// this is the core business logic we expose
impl<'a> Admin<'a> {
    pub const fn new(namespace: &'a str) -> Self {
        Admin(Item::new(namespace))
    }

    pub fn set(&self, deps: DepsMut, admin: Option<HumanAddr>) -> StdResult<()> {
        let admin_raw = maybe_canonical(deps.api, admin)?;
        self.0.save(deps.storage, &admin_raw)
    }

    pub fn get(&self, deps: Deps) -> StdResult<Option<HumanAddr>> {
        let canon = self.0.load(deps.storage)?;
        canon.map(|c| deps.api.human_address(&c)).transpose()
    }

    /// Returns Ok(true) if this is an admin, Ok(false) if not and an Error if
    /// we hit an error with Api or Storage usage
    pub fn is_admin(&self, deps: Deps, caller: &HumanAddr) -> StdResult<bool> {
        match self.0.load(deps.storage)? {
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
            Err(AdminError::NotAdmin {})
        } else {
            Ok(())
        }
    }

    pub fn handle_update_admin(
        &self,
        deps: DepsMut,
        info: MessageInfo,
        new_admin: Option<HumanAddr>,
    ) -> Result<HandleResponse, AdminError> {
        self.assert_admin(deps.as_ref(), &info.sender)?;

        let admin_str = match new_admin.as_ref() {
            Some(admin) => admin.to_string(),
            None => "None".to_string(),
        };
        let attributes = vec![
            attr("action", "update_admin"),
            attr("admin", admin_str),
            attr("sender", info.sender),
        ];

        self.set(deps, new_admin.clone())?;

        Ok(HandleResponse {
            messages: vec![],
            attributes,
            data: None,
        })
    }

    pub fn query_admin(&self, deps: Deps) -> StdResult<AdminResponse> {
        let admin = self.get(deps)?;
        Ok(AdminResponse { admin })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use cosmwasm_std::testing::{mock_dependencies, mock_info};

    #[test]
    fn set_and_get_admin() {
        let mut deps = mock_dependencies(&[]);
        let control = Admin::new("foo");

        // initialize and check
        let admin = Some(HumanAddr::from("admin"));
        control.set(deps.as_mut(), admin.clone()).unwrap();
        let got = control.get(deps.as_ref()).unwrap();
        assert_eq!(admin, got);

        // clear it and check
        control.set(deps.as_mut(), None).unwrap();
        let got = control.get(deps.as_ref()).unwrap();
        assert_eq!(None, got);
    }

    #[test]
    fn admin_checks() {
        let mut deps = mock_dependencies(&[]);

        let control = Admin::new("foo");
        let owner = HumanAddr::from("big boss");
        let imposter = HumanAddr::from("imposter");

        // ensure checks proper with owner set
        control.set(deps.as_mut(), Some(owner.clone())).unwrap();
        assert_eq!(true, control.is_admin(deps.as_ref(), &owner).unwrap());
        assert_eq!(false, control.is_admin(deps.as_ref(), &imposter).unwrap());
        control.assert_admin(deps.as_ref(), &owner).unwrap();
        let err = control.assert_admin(deps.as_ref(), &imposter).unwrap_err();
        assert_eq!(AdminError::NotAdmin {}, err);

        // ensure checks proper with owner None
        control.set(deps.as_mut(), None).unwrap();
        assert_eq!(false, control.is_admin(deps.as_ref(), &owner).unwrap());
        assert_eq!(false, control.is_admin(deps.as_ref(), &imposter).unwrap());
        let err = control.assert_admin(deps.as_ref(), &owner).unwrap_err();
        assert_eq!(AdminError::NotAdmin {}, err);
        let err = control.assert_admin(deps.as_ref(), &imposter).unwrap_err();
        assert_eq!(AdminError::NotAdmin {}, err);
    }

    #[test]
    fn test_handle_query() {
        let mut deps = mock_dependencies(&[]);

        // initial setup
        let control = Admin::new("foo");
        let owner = HumanAddr::from("big boss");
        let imposter = HumanAddr::from("imposter");
        let friend = HumanAddr::from("buddy");
        control.set(deps.as_mut(), Some(owner.clone())).unwrap();

        // query shows results
        let res = control.query_admin(deps.as_ref()).unwrap();
        assert_eq!(Some(owner.clone()), res.admin);

        // imposter cannot update
        let info = mock_info(&imposter, &[]);
        let new_admin = Some(friend.clone());
        let err = control
            .handle_update_admin(deps.as_mut(), info, new_admin.clone())
            .unwrap_err();
        assert_eq!(AdminError::NotAdmin {}, err);

        // owner can update
        let info = mock_info(&owner, &[]);
        let res = control
            .handle_update_admin(deps.as_mut(), info, new_admin)
            .unwrap();
        assert_eq!(0, res.messages.len());

        // query shows results
        let res = control.query_admin(deps.as_ref()).unwrap();
        assert_eq!(Some(friend.clone()), res.admin);
    }
}
