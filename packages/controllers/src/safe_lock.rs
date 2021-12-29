use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;

use cosmwasm_std::{attr, Deps, DepsMut, MessageInfo, Response, StdResult};
use cw_storage_plus::Item;

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct SafeLockResponse {
    pub is_enabled: bool,
}

/// SafeLock is basically a storage helper that indicates if the contract is locked
/// for security reasons.
/// WARNING: SafeLock does not implement any admin or authorisation method,
/// run checks before running safe lock methods.
pub struct SafeLock<'a>(Item<'a, bool>);

impl<'a> SafeLock<'a> {
    pub const fn new(namespace: &'a str) -> Self {
        SafeLock(Item::new(namespace))
    }

    pub fn set(&self, deps: DepsMut, is_enabled: bool) -> StdResult<()> {
        self.0.save(deps.storage, &is_enabled)
    }

    pub fn is_enabled(&self, deps: Deps) -> StdResult<bool> {
        self.0.load(deps.storage)
    }

    pub fn execute_update_safe_lock<C>(
        &self,
        deps: DepsMut,
        _info: MessageInfo,
        is_enabled: bool,
    ) -> StdResult<Response<C>>
    where
        C: Clone + fmt::Debug + PartialEq + JsonSchema,
    {
        self.set(deps, is_enabled)?;

        let attributes = vec![
            attr("action", "update_safe_lock"),
            attr("is_enabled", is_enabled.to_string()),
        ];

        Ok(Response::new().add_attributes(attributes))
    }

    pub fn query_safe_lock(&self, deps: Deps) -> StdResult<SafeLockResponse> {
        let is_enabled = self.0.load(deps.storage)?;
        Ok(SafeLockResponse { is_enabled })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use cosmwasm_std::testing::{mock_dependencies, mock_info};
    use cosmwasm_std::Empty;

    #[test]
    fn test_set_is_enabled() {
        let mut deps = mock_dependencies();
        let lock = SafeLock::new("foo");

        // initialize and check
        lock.set(deps.as_mut(), true).unwrap();
        let got = lock.is_enabled(deps.as_ref()).unwrap();
        assert!(got);

        // clear it and check
        lock.set(deps.as_mut(), false).unwrap();
        let got = lock.is_enabled(deps.as_ref()).unwrap();
        assert!(!got);
    }

    #[test]
    fn test_execute_query() {
        let mut deps = mock_dependencies();

        // initial setup
        let lock = SafeLock::new("foo");
        lock.set(deps.as_mut(), false).unwrap();

        // query shows results
        let res = lock.query_safe_lock(deps.as_ref()).unwrap();
        assert!(!res.is_enabled);

        // update applies
        let info = mock_info("random", &[]);
        let res = lock
            .execute_update_safe_lock::<Empty>(deps.as_mut(), info, true)
            .unwrap();
        assert_eq!(0, res.messages.len());

        // query shows results
        let res = lock.query_safe_lock(deps.as_ref()).unwrap();
        assert!(res.is_enabled);
    }
}
