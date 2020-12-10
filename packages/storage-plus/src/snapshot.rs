#![cfg(feature = "iterator")]

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Order, StdError, StdResult, Storage};

use crate::keys::{EmptyPrefix, PrimaryKey, U64Key};
use crate::map::Map;
use crate::path::Path;
use crate::prefix::Prefix;
use crate::{Bound, Prefixer};

/// Map that maintains a snapshots of one or more checkpoints.
/// We can query historical data as well as current state.
/// What data is snapshotted depends on the Strategy.
pub struct SnapshotMap<'a, K, T> {
    primary: Map<'a, K, T>,

    // maps height to number of checkpoints (only used for selected)
    checkpoints: Map<'a, U64Key, u32>,

    // this stores all changes (key, height). Must differentiate between no data written,
    // and explicit None (just inserted)
    changelog: Map<'a, (K, U64Key), ChangeSet<T>>,

    // How aggressive we are about checkpointing all data
    strategy: Strategy,
}

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub enum Strategy {
    EveryBlock,
    Never,
    /// Only writes for linked blocks - does a few more reads to save some writes.
    /// Probably uses more gas, but less total disk usage.
    ///
    /// Note that you need a trusted source (eg. own contract) to set/remove checkpoints.
    /// Useful when the checkpoint setting happens in the same contract as the snapshotting.
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

    pub fn add_checkpoint(&self, store: &mut dyn Storage, height: u64) -> StdResult<()> {
        self.checkpoints
            .update::<_, StdError>(store, height.into(), |count| {
                Ok(count.unwrap_or_default() + 1)
            })?;
        Ok(())
    }

    pub fn remove_checkpoint(&self, store: &mut dyn Storage, height: u64) -> StdResult<()> {
        let count = self
            .checkpoints
            .may_load(store, height.into())?
            .unwrap_or_default();
        if count <= 1 {
            self.checkpoints.remove(store, height.into());
            Ok(())
        } else {
            self.checkpoints.save(store, height.into(), &(count - 1))
        }
    }
}

impl<'a, K, T> SnapshotMap<'a, K, T>
where
    T: Serialize + DeserializeOwned + Clone,
    K: PrimaryKey<'a> + Prefixer<'a>,
{
    pub fn key(&self, k: K) -> Path<T> {
        self.primary.key(k)
    }

    pub fn prefix(&self, p: K::Prefix) -> Prefix<T> {
        self.primary.prefix(p)
    }

    /// should_checkpoint looks at the strategy and determines if we want to checkpoint
    fn should_checkpoint(&self, store: &dyn Storage, k: &K) -> StdResult<bool> {
        match self.strategy {
            Strategy::EveryBlock => Ok(true),
            Strategy::Never => Ok(false),
            Strategy::Selected => self.should_checkpoint_selected(store, k),
        }
    }

    /// this is just pulled out from above for the selected block
    fn should_checkpoint_selected(&self, store: &dyn Storage, k: &K) -> StdResult<bool> {
        // most recent checkpoint
        let checkpoint = self
            .checkpoints
            .range(store, None, None, Order::Descending)
            .next()
            .transpose()?;
        if let Some((height, _)) = checkpoint {
            // any changelog for the given key since then?
            let start = Bound::inclusive(U64Key::from(height));
            let first = self
                .changelog
                .prefix(k.clone())
                .range(store, Some(start), None, Order::Ascending)
                .next()
                .transpose()?;
            if first.is_none() {
                // there must be at least one open checkpoint and no changelog for the given address since then
                return Ok(true);
            }
        }
        // otherwise, we don't save this
        Ok(false)
    }

    /// load old value and store changelog
    fn write_change(&self, store: &mut dyn Storage, k: K, height: u64) -> StdResult<()> {
        let old = self.primary.may_load(store, k.clone())?;
        self.changelog
            .save(store, (k, U64Key::from(height)), &ChangeSet { old })
    }

    pub fn save(&self, store: &mut dyn Storage, k: K, data: &T, height: u64) -> StdResult<()> {
        if self.should_checkpoint(store, &k)? {
            self.write_change(store, k.clone(), height)?;
        }
        self.primary.save(store, k, data)
    }

    pub fn remove(&self, store: &mut dyn Storage, k: K, height: u64) -> StdResult<()> {
        if self.should_checkpoint(store, &k)? {
            self.write_change(store, k.clone(), height)?;
        }
        self.primary.remove(store, k);
        Ok(())
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
    // Only returns `Ok` if we have the data to be able to give the correct answer
    // (Strategy::EveryBlock or Strategy::Selected and h is registered as checkpoint)
    //
    // If there is no checkpoint for that height, then we return StdError::NotFound
    pub fn may_load_at_height(
        &self,
        store: &dyn Storage,
        k: K,
        height: u64,
    ) -> StdResult<Option<T>> {
        self.assert_checkpointed(store, height)?;

        // this will look for the first snapshot of the given address >= given height
        // If None, there is no snapshot since that time.
        let start = Bound::inclusive(U64Key::new(height));
        let first = self
            .changelog
            .prefix(k.clone())
            .range(store, Some(start), None, Order::Ascending)
            .next();

        if let Some(r) = first {
            // if we found a match, return this last one
            r.map(|(_, v)| v.old)
        } else {
            // otherwise, return current value
            self.may_load(store, k)
        }
    }

    // If there is no checkpoint for that height, then we return StdError::NotFound
    pub fn assert_checkpointed(&self, store: &dyn Storage, height: u64) -> StdResult<()> {
        let has = match self.strategy {
            Strategy::EveryBlock => true,
            Strategy::Never => false,
            Strategy::Selected => self.checkpoints.may_load(store, height.into())?.is_some(),
        };
        match has {
            true => Ok(()),
            false => Err(StdError::not_found("checkpoint")),
        }
    }

    /// Loads the data, perform the specified action, and store the result
    /// in the database. This is shorthand for some common sequences, which may be useful.
    ///
    /// If the data exists, `action(Some(value))` is called. Otherwise `action(None)` is called.
    ///
    /// This is a bit more customized than needed to only read "old" value 1 time, not 2 per naive approach
    pub fn update<A, E>(
        &self,
        store: &mut dyn Storage,
        k: K,
        height: u64,
        action: A,
    ) -> Result<T, E>
    where
        A: FnOnce(Option<T>) -> Result<T, E>,
        E: From<StdError>,
    {
        let input = self.may_load(store, k.clone())?;
        let old = input.clone();

        let output = action(input)?;
        // optimize the save (save the extra read in write_change)
        if self.should_checkpoint(store, &k)? {
            let diff = ChangeSet { old };
            self.changelog
                .save(store, (k.clone(), height.into()), &diff)?;
        }
        self.primary.save(store, k, &output)?;

        Ok(output)
    }
}

// short-cut for simple keys, rather than .prefix(()).range(...)
#[cfg(feature = "iterator")]
impl<'a, K, T> SnapshotMap<'a, K, T>
where
    T: Serialize + DeserializeOwned + Clone,
    K: PrimaryKey<'a> + Prefixer<'a>,
    K::Prefix: EmptyPrefix,
{
    // I would prefer not to copy code from Prefix, but no other way
    // with lifetimes (create Prefix inside function and return ref = no no)
    pub fn range<'c>(
        &self,
        store: &'c dyn Storage,
        min: Option<Bound>,
        max: Option<Bound>,
        order: cosmwasm_std::Order,
    ) -> Box<dyn Iterator<Item = StdResult<cosmwasm_std::KV<T>>> + 'c>
    where
        T: 'c,
    {
        self.prefix(K::Prefix::new()).range(store, min, max, order)
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
        #[allow(clippy::string_lit_as_bytes)]
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
    use cosmwasm_std::testing::MockStorage;

    #[test]
    fn namespace_macro() {
        let check = |names: SnapshotNamespaces| {
            assert_eq!(names.pk, b"demo");
            assert_eq!(names.checkpoints, b"demo__checkpoints");
            assert_eq!(names.changelog, b"demo__changelog");
        };
        // FIXME: we have to do this weird way due to the clippy allow statement
        check(snapshot_names!("demo"));
        // ex. this line fails to compile
        // let names = snapshot_names!("demo");
    }

    type TestMap = SnapshotMap<'static, &'static [u8], u64>;
    const NEVER: TestMap = SnapshotMap::new(snapshot_names!("never"), Strategy::Never);
    const EVERY: TestMap = SnapshotMap::new(snapshot_names!("every"), Strategy::EveryBlock);
    const SELECT: TestMap = SnapshotMap::new(snapshot_names!("select"), Strategy::Selected);

    // Fills a map &[u8] -> u64 with the following writes:
    // 1: A = 5
    // 2: B = 7
    // 3: C = 1, A = 8
    // 4: B = None, C = 13
    // 5: A = None, D = 22
    // Final values -> C = 13, D = 22
    // Values at beginning of 3 -> A = 5, B = 7
    // Values at beginning of 5 -> A = 8, C = 13
    fn init_data(map: &TestMap, storage: &mut dyn Storage) {
        map.save(storage, b"A", &5, 1).unwrap();
        map.save(storage, b"B", &7, 2).unwrap();

        // checkpoint 3
        map.add_checkpoint(storage, 3).unwrap();

        // also use update to set - to ensure this works
        map.save(storage, b"C", &1, 3).unwrap();
        map.update(storage, b"A", 3, |_| -> StdResult<u64> { Ok(8) })
            .unwrap();

        map.remove(storage, b"B", 4).unwrap();
        map.save(storage, b"C", &13, 4).unwrap();

        // checkpoint 5
        map.add_checkpoint(storage, 5).unwrap();
        map.remove(storage, b"A", 5).unwrap();
        map.update(storage, b"D", 5, |_| -> StdResult<u64> { Ok(22) })
            .unwrap();
        // and delete it later (unknown if all data present)
        map.remove_checkpoint(storage, 5).unwrap();
    }

    const FINAL_VALUES: &[(&[u8], Option<u64>)] = &[
        (b"A", None),
        (b"B", None),
        (b"C", Some(13)),
        (b"D", Some(22)),
    ];

    const VALUES_START_3: &[(&[u8], Option<u64>)] =
        &[(b"A", Some(5)), (b"B", Some(7)), (b"C", None), (b"D", None)];

    const VALUES_START_5: &[(&[u8], Option<u64>)] = &[
        (b"A", Some(8)),
        (b"B", None),
        (b"C", Some(13)),
        (b"D", None),
    ];

    fn assert_final_values(map: &TestMap, storage: &dyn Storage) {
        for (k, v) in FINAL_VALUES.iter().cloned() {
            assert_eq!(v, map.may_load(storage, k).unwrap());
        }
    }

    fn assert_values_at_height(
        map: &TestMap,
        storage: &dyn Storage,
        height: u64,
        values: &[(&[u8], Option<u64>)],
    ) {
        for (k, v) in values.iter().cloned() {
            assert_eq!(v, map.may_load_at_height(storage, k, height).unwrap());
        }
    }

    fn assert_missing_checkpoint(map: &TestMap, storage: &dyn Storage, height: u64) {
        for k in &[b"A", b"B", b"C", b"D"] {
            assert!(map.may_load_at_height(storage, *k, height).is_err());
        }
    }

    #[test]
    fn never_works_like_normal_map() {
        let mut storage = MockStorage::new();
        init_data(&NEVER, &mut storage);
        assert_final_values(&NEVER, &storage);

        // historical queries return error
        assert_missing_checkpoint(&NEVER, &storage, 3);
        assert_missing_checkpoint(&NEVER, &storage, 5);
    }

    #[test]
    fn every_blocks_stores_present_and_past() {
        let mut storage = MockStorage::new();
        init_data(&EVERY, &mut storage);
        assert_final_values(&EVERY, &mut storage);

        // historical queries return historical values
        assert_values_at_height(&EVERY, &storage, 3, VALUES_START_3);
        assert_values_at_height(&EVERY, &storage, 5, VALUES_START_5);
    }

    #[test]
    fn selected_shows_3_not_5() {
        let mut storage = MockStorage::new();
        init_data(&SELECT, &mut storage);
        assert_final_values(&SELECT, &mut storage);

        // historical queries return historical values
        assert_values_at_height(&SELECT, &storage, 3, VALUES_START_3);
        // never checkpointed
        assert_missing_checkpoint(&NEVER, &storage, 1);
        // deleted checkpoint
        assert_missing_checkpoint(&NEVER, &storage, 5);
    }
}
