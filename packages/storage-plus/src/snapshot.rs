use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{StdError, StdResult, Storage};

use crate::keys::{PrimaryKey, U64Key};
use crate::map::Map;
use crate::path::Path;
#[cfg(feature = "iterator")]
use crate::prefix::Prefix;

/// Map that maintains a snapshots of one or more checkpoints
pub struct SnapshotMap<'a, K, T> {
    primary: Map<'a, K, T>,

    // maps height to number of checkpoints (only used for selected)
    checkpoints: Map<'a, U64Key, u32>,

    // this stores all changes (key, height). Must differentiate between no data written,
    // and explicit None (just inserted)
    changelog: Map<'a, (K, U64Key), ChangeSet<T>>,

    // TODO: currently only support Never
    strategy: Strategy,
}

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub enum Strategy {
    EveryBlock,
    Never,
    Selected,
}

impl<'a, K, T> SnapshotMap<'a, K, T> {
    /// Usage: SnapshotMap::new(snapshot_names!("foobar"), Strategy::EveryBlock)
    pub const fn new(namespaces: SnapshotNamespaces<'a>, strategy: Strategy) -> Self {
        SnapshotMap {
            primary: Map::new(namespaces.pk),
            checkpoints: Map::new(namespaces.checkpoints),
            changelog: Map::new(namespaces.changelog),
            strategy,
        }
    }
}

impl<'a, K, T> SnapshotMap<'a, K, T>
where
    T: Serialize + DeserializeOwned,
    K: PrimaryKey<'a>,
{
    pub fn key(&self, k: K) -> Path<T> {
        self.primary.key(k)
    }

    #[cfg(feature = "iterator")]
    pub fn prefix(&self, p: K::Prefix) -> Prefix<T> {
        self.primary.prefix(p)
    }

    pub fn save(&self, store: &mut dyn Storage, k: K, data: &T, height: u64) -> StdResult<()> {
        // TODO: check strategy
        unimplemented!();
        // self.key(k).save(store, data)
    }

    pub fn remove(&self, store: &mut dyn Storage, k: K, height: u64) {
        // TODO: check strategy
        unimplemented!();
        // self.key(k).remove(store)
    }

    /// load will return an error if no data is set at the given key, or on parse error
    pub fn load(&self, store: &dyn Storage, k: K) -> StdResult<T> {
        self.primary.load(store, k)
    }

    /// may_load will parse the data stored at the key if present, returns Ok(None) if no data there.
    /// returns an error on issues parsing
    pub fn may_load(&self, store: &dyn Storage, k: K) -> StdResult<Option<T>> {
        self.primary.may_load(store, k)
    }

    // may_load_at_height reads historical data from given checkpoints.
    // only guaranteed to give correct data if Strategy::EveryBlock or
    // Strategy::Selected and h element of checkpoint heights
    pub fn may_load_at_height(
        &self,
        store: &dyn Storage,
        k: K,
        height: u64,
    ) -> StdResult<Option<T>> {
        // TODO: check strategy
        unimplemented!();
        // self.key(k).may_load(store)
    }

    /// Loads the data, perform the specified action, and store the result
    /// in the database. This is shorthand for some common sequences, which may be useful.
    ///
    /// If the data exists, `action(Some(value))` is called. Otherwise `action(None)` is called.
    pub fn update<A, E>(&self, store: &mut dyn Storage, k: K, action: A) -> Result<T, E>
    where
        A: FnOnce(Option<T>) -> Result<T, E>,
        E: From<StdError>,
    {
        // TODO
        unimplemented!();
    }
}

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
struct ChangeSet<T> {
    pub old: Option<T>,
}

pub struct SnapshotNamespaces<'a> {
    pub pk: &'a [u8],
    pub checkpoints: &'a [u8],
    pub changelog: &'a [u8],
}

#[macro_export]
macro_rules! snapshot_names {
    ($var:expr) => {
        SnapshotNamespaces {
            pk: $var.as_bytes(),
            checkpoints: concat!($var, "__checkpoints").as_bytes(),
            changelog: concat!($var, "__changelog").as_bytes(),
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn namespace_macro() {
        let names = snapshot_names!("demo");
        assert_eq!(names.pk, b"demo");
        assert_eq!(names.checkpoints, b"demo__checkpoints");
        assert_eq!(names.changelog, b"demo__changelog");
    }
}
