use serde::de::DeserializeOwned;
use serde::Serialize;

use cosmwasm_std::{StdError, StdResult, Storage};

use crate::bound::PrefixBound;
use crate::de::KeyDeserialize;
use crate::iter_helpers::deserialize_kv;
use crate::keys::PrimaryKey;
use crate::map::Map;
use crate::path::Path;
use crate::prefix::{namespaced_prefix_range, Prefix};
use crate::snapshot::{ChangeSet, Snapshot};
use crate::{Bound, Prefixer, Strategy};

/// Map that maintains a snapshots of one or more checkpoints.
/// We can query historical data as well as current state.
/// What data is snapshotted depends on the Strategy.
pub struct SnapshotMap<'a, K, T> {
    primary: Map<'a, K, T>,
    snapshots: Snapshot<'a, K, T>,
}

impl<'a, K, T> SnapshotMap<'a, K, T> {
    /// Example:
    ///
    /// ```rust
    /// use cw_storage_plus::{SnapshotMap, Strategy};
    ///
    /// SnapshotMap::<&[u8], &str>::new(
    ///     "never",
    ///     "never__check",
    ///     "never__change",
    ///     Strategy::EveryBlock
    /// );
    /// ```
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

    pub fn changelog(&self) -> &Map<'a, (K, u64), ChangeSet<T>> {
        &self.snapshots.changelog
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
    K: PrimaryKey<'a> + Prefixer<'a> + KeyDeserialize,
{
    pub fn key(&self, k: K) -> Path<T> {
        self.primary.key(k)
    }

    fn no_prefix_raw(&self) -> Prefix<Vec<u8>, T, K> {
        self.primary.no_prefix_raw()
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

// short-cut for simple keys, rather than .prefix(()).range_raw(...)
impl<'a, K, T> SnapshotMap<'a, K, T>
where
    T: Serialize + DeserializeOwned + Clone,
    K: PrimaryKey<'a> + Prefixer<'a> + KeyDeserialize,
{
    // I would prefer not to copy code from Prefix, but no other way
    // with lifetimes (create Prefix inside function and return ref = no no)
    pub fn range_raw<'c>(
        &self,
        store: &'c dyn Storage,
        min: Option<Bound<'a, K>>,
        max: Option<Bound<'a, K>>,
        order: cosmwasm_std::Order,
    ) -> Box<dyn Iterator<Item = StdResult<cosmwasm_std::Record<T>>> + 'c>
    where
        T: 'c,
    {
        self.no_prefix_raw().range_raw(store, min, max, order)
    }

    pub fn keys_raw<'c>(
        &self,
        store: &'c dyn Storage,
        min: Option<Bound<'a, K>>,
        max: Option<Bound<'a, K>>,
        order: cosmwasm_std::Order,
    ) -> Box<dyn Iterator<Item = Vec<u8>> + 'c>
    where
        T: 'c,
    {
        self.no_prefix_raw().keys_raw(store, min, max, order)
    }
}

#[cfg(feature = "iterator")]
impl<'a, K, T> SnapshotMap<'a, K, T>
where
    T: Serialize + DeserializeOwned,
    K: PrimaryKey<'a> + KeyDeserialize,
{
    /// While `range` over a `prefix` fixes the prefix to one element and iterates over the
    /// remaining, `prefix_range` accepts bounds for the lowest and highest elements of the
    /// `Prefix` itself, and iterates over those (inclusively or exclusively, depending on
    /// `PrefixBound`).
    /// There are some issues that distinguish these two, and blindly casting to `Vec<u8>` doesn't
    /// solve them.
    pub fn prefix_range<'c>(
        &self,
        store: &'c dyn Storage,
        min: Option<PrefixBound<'a, K::Prefix>>,
        max: Option<PrefixBound<'a, K::Prefix>>,
        order: cosmwasm_std::Order,
    ) -> Box<dyn Iterator<Item = StdResult<(K::Output, T)>> + 'c>
    where
        T: 'c,
        'a: 'c,
        K: 'c,
        K::Output: 'static,
    {
        let mapped = namespaced_prefix_range(store, self.primary.namespace(), min, max, order)
            .map(deserialize_kv::<K, T>);
        Box::new(mapped)
    }

    pub fn range<'c>(
        &self,
        store: &'c dyn Storage,
        min: Option<Bound<'a, K>>,
        max: Option<Bound<'a, K>>,
        order: cosmwasm_std::Order,
    ) -> Box<dyn Iterator<Item = StdResult<(K::Output, T)>> + 'c>
    where
        T: 'c,
        K::Output: 'static,
    {
        self.no_prefix().range(store, min, max, order)
    }

    pub fn keys<'c>(
        &self,
        store: &'c dyn Storage,
        min: Option<Bound<'a, K>>,
        max: Option<Bound<'a, K>>,
        order: cosmwasm_std::Order,
    ) -> Box<dyn Iterator<Item = StdResult<K::Output>> + 'c>
    where
        T: 'c,
        K::Output: 'static,
    {
        self.no_prefix().keys(store, min, max, order)
    }

    pub fn prefix(&self, p: K::Prefix) -> Prefix<K::Suffix, T, K::Suffix> {
        Prefix::new(self.primary.namespace(), &p.prefix())
    }

    pub fn sub_prefix(&self, p: K::SubPrefix) -> Prefix<K::SuperSuffix, T, K::SuperSuffix> {
        Prefix::new(self.primary.namespace(), &p.prefix())
    }

    fn no_prefix(&self) -> Prefix<K, T, K> {
        Prefix::new(self.primary.namespace(), &[])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::MockStorage;

    type TestMap = SnapshotMap<'static, &'static str, u64>;
    type TestMapCompositeKey = SnapshotMap<'static, (&'static str, &'static str), u64>;

    const NEVER: TestMap =
        SnapshotMap::new("never", "never__check", "never__change", Strategy::Never);
    const EVERY: TestMap = SnapshotMap::new(
        "every",
        "every__check",
        "every__change",
        Strategy::EveryBlock,
    );
    const EVERY_COMPOSITE_KEY: TestMapCompositeKey = SnapshotMap::new(
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
        map.save(storage, "A", &5, 1).unwrap();
        map.save(storage, "B", &7, 2).unwrap();

        // checkpoint 3
        map.add_checkpoint(storage, 3).unwrap();

        // also use update to set - to ensure this works
        map.save(storage, "C", &1, 3).unwrap();
        map.update(storage, "A", 3, |_| -> StdResult<u64> { Ok(8) })
            .unwrap();

        map.remove(storage, "B", 4).unwrap();
        map.save(storage, "C", &13, 4).unwrap();

        // checkpoint 5
        map.add_checkpoint(storage, 5).unwrap();
        map.remove(storage, "A", 5).unwrap();
        map.update(storage, "D", 5, |_| -> StdResult<u64> { Ok(22) })
            .unwrap();
        // and delete it later (unknown if all data present)
        map.remove_checkpoint(storage, 5).unwrap();
    }

    const FINAL_VALUES: &[(&str, Option<u64>)] =
        &[("A", None), ("B", None), ("C", Some(13)), ("D", Some(22))];

    const VALUES_START_3: &[(&str, Option<u64>)] =
        &[("A", Some(5)), ("B", Some(7)), ("C", None), ("D", None)];

    const VALUES_START_5: &[(&str, Option<u64>)] =
        &[("A", Some(8)), ("B", None), ("C", Some(13)), ("D", None)];

    // Same as `init_data`, but we have a composite key for testing range.
    fn init_data_composite_key(map: &TestMapCompositeKey, storage: &mut dyn Storage) {
        map.save(storage, ("A", "B"), &5, 1).unwrap();
        map.save(storage, ("B", "A"), &7, 2).unwrap();

        // checkpoint 3
        map.add_checkpoint(storage, 3).unwrap();

        // also use update to set - to ensure this works
        map.save(storage, ("B", "B"), &1, 3).unwrap();
        map.update(storage, ("A", "B"), 3, |_| -> StdResult<u64> { Ok(8) })
            .unwrap();

        map.remove(storage, ("B", "A"), 4).unwrap();
        map.save(storage, ("B", "B"), &13, 4).unwrap();

        // checkpoint 5
        map.add_checkpoint(storage, 5).unwrap();
        map.remove(storage, ("A", "B"), 5).unwrap();
        map.update(storage, ("C", "A"), 5, |_| -> StdResult<u64> { Ok(22) })
            .unwrap();
        // and delete it later (unknown if all data present)
        map.remove_checkpoint(storage, 5).unwrap();
    }

    fn assert_final_values(map: &TestMap, storage: &dyn Storage) {
        for (k, v) in FINAL_VALUES.iter().cloned() {
            assert_eq!(v, map.may_load(storage, k).unwrap());
        }
    }

    fn assert_values_at_height(
        map: &TestMap,
        storage: &dyn Storage,
        height: u64,
        values: &[(&str, Option<u64>)],
    ) {
        for (k, v) in values.iter().cloned() {
            assert_eq!(v, map.may_load_at_height(storage, k, height).unwrap());
        }
    }

    fn assert_missing_checkpoint(map: &TestMap, storage: &dyn Storage, height: u64) {
        for k in &["A", "B", "C", "D"] {
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
        EVERY.save(&mut storage, "A", &5, 1).unwrap();
        EVERY.save(&mut storage, "B", &7, 2).unwrap();
        EVERY.save(&mut storage, "C", &2, 2).unwrap();

        // update and save - A query at 3 => 5, at 4 => 12
        EVERY
            .update(&mut storage, "A", 3, |_| -> StdResult<u64> { Ok(9) })
            .unwrap();
        EVERY.save(&mut storage, "A", &12, 3).unwrap();
        assert_eq!(Some(5), EVERY.may_load_at_height(&storage, "A", 2).unwrap());
        assert_eq!(Some(5), EVERY.may_load_at_height(&storage, "A", 3).unwrap());
        assert_eq!(
            Some(12),
            EVERY.may_load_at_height(&storage, "A", 4).unwrap()
        );

        // save and remove - B query at 4 => 7, at 5 => None
        EVERY.save(&mut storage, "B", &17, 4).unwrap();
        EVERY.remove(&mut storage, "B", 4).unwrap();
        assert_eq!(Some(7), EVERY.may_load_at_height(&storage, "B", 3).unwrap());
        assert_eq!(Some(7), EVERY.may_load_at_height(&storage, "B", 4).unwrap());
        assert_eq!(None, EVERY.may_load_at_height(&storage, "B", 5).unwrap());

        // remove and update - C query at 5 => 2, at 6 => 16
        EVERY.remove(&mut storage, "C", 5).unwrap();
        EVERY
            .update(&mut storage, "C", 5, |_| -> StdResult<u64> { Ok(16) })
            .unwrap();
        assert_eq!(Some(2), EVERY.may_load_at_height(&storage, "C", 4).unwrap());
        assert_eq!(Some(2), EVERY.may_load_at_height(&storage, "C", 5).unwrap());
        assert_eq!(
            Some(16),
            EVERY.may_load_at_height(&storage, "C", 6).unwrap()
        );
    }

    #[test]
    #[cfg(feature = "iterator")]
    fn changelog_range_works() {
        use cosmwasm_std::Order;

        let mut store = MockStorage::new();

        // simple data for testing
        EVERY.save(&mut store, "A", &5, 1).unwrap();
        EVERY.save(&mut store, "B", &7, 2).unwrap();
        EVERY
            .update(&mut store, "A", 3, |_| -> StdResult<u64> { Ok(8) })
            .unwrap();
        EVERY.remove(&mut store, "B", 4).unwrap();

        // let's try to iterate over the changelog
        let all: StdResult<Vec<_>> = EVERY
            .changelog()
            .range(&store, None, None, Order::Ascending)
            .collect();
        let all = all.unwrap();
        assert_eq!(4, all.len());
        assert_eq!(
            all,
            vec![
                (("A".into(), 1), ChangeSet { old: None }),
                (("A".into(), 3), ChangeSet { old: Some(5) }),
                (("B".into(), 2), ChangeSet { old: None }),
                (("B".into(), 4), ChangeSet { old: Some(7) })
            ]
        );

        // let's try to iterate over a changelog key/prefix
        let all: StdResult<Vec<_>> = EVERY
            .changelog()
            .prefix("B")
            .range(&store, None, None, Order::Ascending)
            .collect();
        let all = all.unwrap();
        assert_eq!(2, all.len());
        assert_eq!(
            all,
            vec![
                (2, ChangeSet { old: None }),
                (4, ChangeSet { old: Some(7) })
            ]
        );

        // let's try to iterate over a changelog prefixed range
        let all: StdResult<Vec<_>> = EVERY
            .changelog()
            .prefix("A")
            .range(&store, Some(Bound::inclusive(3u64)), None, Order::Ascending)
            .collect();
        let all = all.unwrap();
        assert_eq!(1, all.len());
        assert_eq!(all, vec![(3, ChangeSet { old: Some(5) }),]);
    }

    #[test]
    #[cfg(feature = "iterator")]
    fn range_simple_string_key() {
        use cosmwasm_std::Order;

        let mut store = MockStorage::new();
        init_data(&EVERY, &mut store);

        // let's try to iterate!
        let all: StdResult<Vec<_>> = EVERY.range(&store, None, None, Order::Ascending).collect();
        let all = all.unwrap();
        assert_eq!(2, all.len());
        assert_eq!(all, vec![("C".into(), 13), ("D".into(), 22)]);

        // let's try to iterate over a range
        let all: StdResult<Vec<_>> = EVERY
            .range(&store, Some(Bound::inclusive("C")), None, Order::Ascending)
            .collect();
        let all = all.unwrap();
        assert_eq!(2, all.len());
        assert_eq!(all, vec![("C".into(), 13), ("D".into(), 22)]);

        // let's try to iterate over a more restrictive range
        let all: StdResult<Vec<_>> = EVERY
            .range(&store, Some(Bound::inclusive("D")), None, Order::Ascending)
            .collect();
        let all = all.unwrap();
        assert_eq!(1, all.len());
        assert_eq!(all, vec![("D".into(), 22)]);
    }

    #[test]
    #[cfg(feature = "iterator")]
    fn range_composite_key() {
        use cosmwasm_std::Order;

        let mut store = MockStorage::new();
        init_data_composite_key(&EVERY_COMPOSITE_KEY, &mut store);

        // let's try to iterate!
        let all: StdResult<Vec<_>> = EVERY_COMPOSITE_KEY
            .range(&store, None, None, Order::Ascending)
            .collect();
        let all = all.unwrap();
        assert_eq!(2, all.len());
        assert_eq!(
            all,
            vec![
                (("B".into(), "B".into()), 13),
                (("C".into(), "A".into()), 22)
            ]
        );
    }

    #[test]
    #[cfg(feature = "iterator")]
    fn prefix_range_composite_key() {
        use cosmwasm_std::Order;

        let mut store = MockStorage::new();
        init_data_composite_key(&EVERY_COMPOSITE_KEY, &mut store);

        // let's prefix-range and iterate
        let all: StdResult<Vec<_>> = EVERY_COMPOSITE_KEY
            .prefix_range(
                &store,
                None,
                Some(PrefixBound::exclusive("C")),
                Order::Descending,
            )
            .collect();
        let all = all.unwrap();
        assert_eq!(1, all.len());
        assert_eq!(all, vec![(("B".into(), "B".into()), 13)]);
    }

    #[test]
    #[cfg(feature = "iterator")]
    fn prefix_composite_key() {
        use cosmwasm_std::Order;

        let mut store = MockStorage::new();
        init_data_composite_key(&EVERY_COMPOSITE_KEY, &mut store);

        // let's prefix and iterate
        let all: StdResult<Vec<_>> = EVERY_COMPOSITE_KEY
            .prefix("C")
            .range(&store, None, None, Order::Ascending)
            .collect();
        let all = all.unwrap();
        assert_eq!(1, all.len());
        assert_eq!(all, vec![("A".into(), 22),]);
    }

    #[test]
    #[cfg(feature = "iterator")]
    fn sub_prefix_composite_key() {
        use cosmwasm_std::Order;

        let mut store = MockStorage::new();
        init_data_composite_key(&EVERY_COMPOSITE_KEY, &mut store);

        // Let's sub-prefix and iterate.
        // This is similar to calling range() directly, but added here for completeness /
        // sub_prefix type checks
        let all: StdResult<Vec<_>> = EVERY_COMPOSITE_KEY
            .sub_prefix(())
            .range(&store, None, None, Order::Ascending)
            .collect();
        let all = all.unwrap();
        assert_eq!(2, all.len());
        assert_eq!(
            all,
            vec![
                (("B".into(), "B".into()), 13),
                (("C".into(), "A".into()), 22)
            ]
        );
    }
}
