use std::marker::PhantomData;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Empty};
use cw_storage_plus::Item;

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, JsonSchema, Debug, Default)]
pub struct AdminList {
    pub admins: Vec<Addr>,
    pub mutable: bool,
}

impl AdminList {
    /// returns true if the address is a registered admin
    pub fn is_admin(&self, addr: &str) -> bool {
        self.admins.iter().any(|a| a.as_ref() == addr)
    }

    /// returns true if the address is a registered admin and the config is mutable
    pub fn can_modify(&self, addr: &str) -> bool {
        self.mutable && self.is_admin(addr)
    }
}

pub struct Cw1WhitelistContract<T> {
    // I am pretty sure that just form this with some proper hint attributes it would be possible
    // to provide helpers for raw queries, this might be fun idea
    pub(crate) admin_list: Item<'static, AdminList>,
    _msg: PhantomData<T>,
}

impl Cw1WhitelistContract<Empty> {
    // Native form of this contract as it is to be created in entry points
    pub const fn native() -> Self {
        Self::new()
    }
}

impl<T> Cw1WhitelistContract<T> {
    pub const fn new() -> Self {
        Self {
            admin_list: Item::new("admin_list"),
            _msg: PhantomData,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_admin() {
        let admins: Vec<_> = vec!["bob", "paul", "john"]
            .into_iter()
            .map(Addr::unchecked)
            .collect();
        let config = AdminList {
            admins: admins.clone(),
            mutable: false,
        };

        assert!(config.is_admin(admins[0].as_ref()));
        assert!(config.is_admin(admins[2].as_ref()));
        assert!(!config.is_admin("other"));
    }

    #[test]
    fn can_modify() {
        let alice = Addr::unchecked("alice");
        let bob = Addr::unchecked("bob");

        // admin can modify mutable contract
        let config = AdminList {
            admins: vec![bob.clone()],
            mutable: true,
        };
        assert!(!config.can_modify(alice.as_ref()));
        assert!(config.can_modify(bob.as_ref()));

        // no one can modify an immutable contract
        let config = AdminList {
            admins: vec![alice.clone()],
            mutable: false,
        };
        assert!(!config.can_modify(alice.as_ref()));
        assert!(!config.can_modify(bob.as_ref()));
    }
}
