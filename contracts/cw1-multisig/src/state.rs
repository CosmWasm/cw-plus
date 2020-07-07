use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{CanonicalAddr, ReadonlyStorage, Storage};
use cosmwasm_storage::{singleton, singleton_read, ReadonlySingleton, Singleton};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug, Default)]
pub struct Config {
    pub admins: Vec<CanonicalAddr>,
    pub mutable: bool,
}

impl Config {
    /// returns true if the address is a registered admin
    pub fn is_admin(&self, addr: &CanonicalAddr) -> bool {
        self.admins.iter().any(|a| a == addr)
    }

    /// returns true if the address is a registered admin and the config is mutable
    pub fn can_modify(&self, addr: &CanonicalAddr) -> bool {
        self.mutable && self.is_admin(addr)
    }
}

pub const CONFIG_KEY: &[u8] = b"config";

// config is all config information
pub fn config<S: Storage>(storage: &mut S) -> Singleton<S, Config> {
    singleton(storage, CONFIG_KEY)
}

pub fn config_read<S: ReadonlyStorage>(storage: &S) -> ReadonlySingleton<S, Config> {
    singleton_read(storage, CONFIG_KEY)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::MockApi;
    use cosmwasm_std::{Api, HumanAddr};

    #[test]
    fn is_admin() {
        let api = MockApi::new(20);
        let admins: Vec<_> = ["bob", "paul", "john"]
            .iter()
            .map(|name| api.canonical_address(&HumanAddr::from(name)).unwrap())
            .collect();
        let config = Config {
            admins: admins.clone(),
            mutable: false,
        };

        assert!(config.is_admin(&admins[0]));
        assert!(config.is_admin(&admins[2]));
        let other = api.canonical_address(&HumanAddr::from("other")).unwrap();
        assert!(!config.is_admin(&other));
    }

    #[test]
    fn can_modify() {
        let api = MockApi::new(20);
        let alice = api.canonical_address(&HumanAddr::from("alice")).unwrap();
        let bob = api.canonical_address(&HumanAddr::from("bob")).unwrap();

        // admin can modify mutable contract
        let config = Config {
            admins: vec![bob.clone()],
            mutable: true,
        };
        assert!(!config.can_modify(&alice));
        assert!(config.can_modify(&bob));

        // no one can modify an immutable contract
        let config = Config {
            admins: vec![alice.clone()],
            mutable: false,
        };
        assert!(!config.can_modify(&alice));
        assert!(!config.can_modify(&bob));
    }
}
