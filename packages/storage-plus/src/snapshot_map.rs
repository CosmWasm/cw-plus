#![cfg(feature = "iterator")]

use serde::de::DeserializeOwned;
use serde::Serialize;

use cosmwasm_std::{StdError, StdResult, Storage};

use crate::keys::{EmptyPrefix, PrimaryKey};
use crate::map::Map;
use crate::path::Path;
use crate::prefix::Prefix;
use crate::snapshot::Snapshot;
use crate::{Bound, Prefixer, Strategy};

/// Map that maintains a snapshots of one or more checkpoints.
/// We can query historical data as well as current state.
/// What data is snapshotted depends on the Strategy.
pub struct SnapshotMap<'a, K, T> {
    primary: Map<'a, K, T>,
    snapshots: Snapshot<'a, K, T>,
}

impl<'a, K, T> SnapshotMap<'a, K, T> {
    /// Usage: SnapshotMap::new(snapshot_names!("foobar"), Strategy::EveryBlock)
    pub const fn new(
        pk: &'a str,
        checkpoints: &'a str,
        changelog: &'a str,
        strategy: Strategy,
    ) -> Self {
        SnapshotMap {
            primary: Map::new(pk),
            snapshots: Snapshot::new(checkpoints, changelog, strategy),
        }
    }
}

impl<'a, K, T> SnapshotMap<'a, K, T>
where
    T: Serialize + DeserializeOwned + Clone,
    K: PrimaryKey<'a> + Prefixer<'a>,
{
    pub fn add_checkpoint(&self, store: &mut dyn Storage, height: u64) -> StdResult<()> {
        self.snapshots.add_checkpoint(store, height)
    }

    pub fn remove_checkpoint(&self, store: &mut dyn Storage, height: u64) -> StdResult<()> {
        self.snapshots.remove_checkpoint(store, height)
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

    pub fn sub_prefix(&self, p: K::SubPrefix) -> Prefix<T> {
        self.primary.sub_prefix(p)
    }

    /// load old value and store changelog
    fn write_change(&self, store: &mut dyn Storage, k: K, height: u64) -> StdResult<()> {
        // if there is already data in the changelog for this key and block, do not write more
        if self.snapshots.has_changelog(store, k.clone(), height)? {
            return Ok(());
        }
        // otherwise, store the previous value
        let old = self.primary.may_load(store, k.clone())?;
        self.snapshots.write_changelog(store, k, height, old)
    }

    pub fn save(&self, store: &mut dyn Storage, k: K, data: &T, height: u64) -> StdResult<()> {
        if self.snapshots.should_checkpoint(store, &k)? {
            self.write_change(store, k.clone(), height)?;
        }
        self.primary.save(store, k, data)
    }

    pub fn remove(&self, store: &mut dyn Storage, k: K, height: u64) -> StdResult<()> {
        if self.snapshots.should_checkpoint(store, &k)? {
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

    pub fn may_load_at_height(
        &self,
        store: &dyn Storage,
        k: K,
        height: u64,
    ) -> StdResult<Option<T>> {
        let snapshot = self
            .snapshots
            .may_load_at_height(store, k.clone(), height)?;

        if let Some(r) = snapshot {
            Ok(r)
        } else {
            // otherwise, return current value
            self.may_load(store, k)
        }
    }

    pub fn assert_checkpointed(&self, store: &dyn Storage, height: u64) -> StdResult<()> {
        self.snapshots.assert_checkpointed(store, height)
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
        let output = action(input)?;
        self.save(store, k, &output, height)?;
        Ok(output)
    }
}

// short-cut for simple keys, rather than .prefix(()).range(...)
#[cfg(feature = "iterator")]
impl<'a, K, T> SnapshotMap<'a, K, T>
where
    T: Serialize + DeserializeOwned + Clone,
    K: PrimaryKey<'a> + Prefixer<'a>,
    K::SubPrefix: EmptyPrefix,
{
    // I would prefer not to copy code from Prefix, but no other way
    // with lifetimes (create Prefix inside function and return ref = no no)
    pub fn range<'c>(
        &self,
        store: &'c dyn Storage,
        min: Option<Bound>,
        max: Option<Bound>,
        order: cosmwasm_std::Order,
    ) -> Box<dyn Iterator<Item = StdResult<cosmwasm_std::Pair<T>>> + 'c>
    where
        T: 'c,
    {
        self.sub_prefix(K::SubPrefix::new())
            .range(store, min, max, order)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::MockStorage;

    type TestMap = SnapshotMap<'static, &'static [u8], u64>;
    const NEVER: TestMap =
        SnapshotMap::new("never", "never__check", "never__change", Strategy::Never);
    const EVERY: TestMap = SnapshotMap::new(
        "every",
        "every__check",
        "every__change",
        Strategy::EveryBlock,
    );
    const SELECT: TestMap = SnapshotMap::new(
        "select",
        "select__check",
        "select__change",
        Strategy::Selected,
    );

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
        assert_final_values(&EVERY, &storage);

        // historical queries return historical values
        assert_values_at_height(&EVERY, &storage, 3, VALUES_START_3);
        assert_values_at_height(&EVERY, &storage, 5, VALUES_START_5);
    }

    #[test]
    fn selected_shows_3_not_5() {
        let mut storage = MockStorage::new();
        init_data(&SELECT, &mut storage);
        assert_final_values(&SELECT, &storage);

        // historical queries return historical values
        assert_values_at_height(&SELECT, &storage, 3, VALUES_START_3);
        // never checkpointed
        assert_missing_checkpoint(&NEVER, &storage, 1);
        // deleted checkpoint
        assert_missing_checkpoint(&NEVER, &storage, 5);
    }

    #[test]
    fn handle_multiple_writes_in_one_block() {
        let mut storage = MockStorage::new();

        println!("SETUP");
        EVERY.save(&mut storage, b"A", &5, 1).unwrap();
        EVERY.save(&mut storage, b"B", &7, 2).unwrap();
        EVERY.save(&mut storage, b"C", &2, 2).unwrap();

        // update and save - A query at 3 => 5, at 4 => 12
        EVERY
            .update(&mut storage, b"A", 3, |_| -> StdResult<u64> { Ok(9) })
            .unwrap();
        EVERY.save(&mut storage, b"A", &12, 3).unwrap();
        assert_eq!(
            Some(5),
            EVERY.may_load_at_height(&storage, b"A", 2).unwrap()
        );
        assert_eq!(
            Some(5),
            EVERY.may_load_at_height(&storage, b"A", 3).unwrap()
        );
        assert_eq!(
            Some(12),
            EVERY.may_load_at_height(&storage, b"A", 4).unwrap()
        );

        // save and remove - B query at 4 => 7, at 5 => None
        EVERY.save(&mut storage, b"B", &17, 4).unwrap();
        EVERY.remove(&mut storage, b"B", 4).unwrap();
        assert_eq!(
            Some(7),
            EVERY.may_load_at_height(&storage, b"B", 3).unwrap()
        );
        assert_eq!(
            Some(7),
            EVERY.may_load_at_height(&storage, b"B", 4).unwrap()
        );
        assert_eq!(None, EVERY.may_load_at_height(&storage, b"B", 5).unwrap());

        // remove and update - C query at 5 => 2, at 6 => 16
        EVERY.remove(&mut storage, b"C", 5).unwrap();
        EVERY
            .update(&mut storage, b"C", 5, |_| -> StdResult<u64> { Ok(16) })
            .unwrap();
        assert_eq!(
            Some(2),
            EVERY.may_load_at_height(&storage, b"C", 4).unwrap()
        );
        assert_eq!(
            Some(2),
            EVERY.may_load_at_height(&storage, b"C", 5).unwrap()
        );
        assert_eq!(
            Some(16),
            EVERY.may_load_at_height(&storage, b"C", 6).unwrap()
        );
    }
}
