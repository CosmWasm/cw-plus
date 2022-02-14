// this module requires iterator to be useful at all
#![cfg(feature = "iterator")]

use cosmwasm_std::{StdError, StdResult, Storage};
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::de::KeyDeserialize;
use crate::iter_helpers::deserialize_kv;
use crate::keys::{Prefixer, PrimaryKey};
use crate::prefix::{namespaced_prefix_range, Prefix};
use crate::snapshot::{ChangeSet, SnapshotMap};
use crate::PrefixBound;
use crate::{Bound, IndexList, Map, Path, Strategy};

/// `IndexedSnapshotMap` works like a `SnapshotMap` but has a secondary index
pub struct IndexedSnapshotMap<'a, K, T, I> {
    pk_namespace: &'a [u8],
    primary: SnapshotMap<'a, K, T>,
    /// This is meant to be read directly to get the proper types, like:
    /// map.idx.owner.items(...)
    pub idx: I,
}

impl<'a, K, T, I> IndexedSnapshotMap<'a, K, T, I> {
    /// Examples:
    ///
    /// ```rust
    /// use cw_storage_plus::{IndexedSnapshotMap, Strategy, UniqueIndex};
    ///
    /// #[derive(PartialEq, Debug, Clone)]
    /// struct Data {
    ///     pub name: String,
    ///     pub age: u32,
    /// }
    ///
    /// let indexes = UniqueIndex::new(|d: &Data| d.age, "data__age");
    ///
    /// IndexedSnapshotMap::<&[u8], Data, UniqueIndex<u32, Data>>::new(
    ///     "data",
    ///     "checkpoints",
    ///     "changelog",
    ///     Strategy::EveryBlock,
    ///     indexes,
    /// );
    /// ```
    pub fn new(
        pk_namespace: &'a str,
        checkpoints: &'a str,
        changelog: &'a str,
        strategy: Strategy,
        indexes: I,
    ) -> Self {
        IndexedSnapshotMap {
            pk_namespace: pk_namespace.as_bytes(),
            primary: SnapshotMap::new(pk_namespace, checkpoints, changelog, strategy),
            idx: indexes,
        }
    }

    pub fn changelog(&self) -> &Map<'a, (K, u64), ChangeSet<T>> {
        self.primary.changelog()
    }
}

impl<'a, K, T, I> IndexedSnapshotMap<'a, K, T, I>
where
    T: Serialize + DeserializeOwned + Clone,
    K: PrimaryKey<'a> + Prefixer<'a> + KeyDeserialize,
    I: IndexList<T>,
{
    pub fn add_checkpoint(&self, store: &mut dyn Storage, height: u64) -> StdResult<()> {
        self.primary.add_checkpoint(store, height)
    }

    pub fn remove_checkpoint(&self, store: &mut dyn Storage, height: u64) -> StdResult<()> {
        self.primary.remove_checkpoint(store, height)
    }

    pub fn may_load_at_height(
        &self,
        store: &dyn Storage,
        k: K,
        height: u64,
    ) -> StdResult<Option<T>> {
        self.primary.may_load_at_height(store, k, height)
    }

    pub fn assert_checkpointed(&self, store: &dyn Storage, height: u64) -> StdResult<()> {
        self.primary.assert_checkpointed(store, height)
    }

    pub fn key(&self, k: K) -> Path<T> {
        self.primary.key(k)
    }
}

impl<'a, K, T, I> IndexedSnapshotMap<'a, K, T, I>
where
    K: PrimaryKey<'a> + Prefixer<'a> + KeyDeserialize,
    T: Serialize + DeserializeOwned + Clone,
    I: IndexList<T>,
{
    /// save will serialize the model and store, returns an error on serialization issues.
    /// this must load the old value to update the indexes properly
    /// if you loaded the old value earlier in the same function, use replace to avoid needless db reads
    pub fn save(&self, store: &mut dyn Storage, key: K, data: &T, height: u64) -> StdResult<()> {
        let old_data = self.may_load(store, key.clone())?;
        self.replace(store, key, Some(data), old_data.as_ref(), height)
    }

    pub fn remove(&self, store: &mut dyn Storage, key: K, height: u64) -> StdResult<()> {
        let old_data = self.may_load(store, key.clone())?;
        self.replace(store, key, None, old_data.as_ref(), height)
    }

    /// replace writes data to key. old_data must be the current stored value (from a previous load)
    /// and is used to properly update the index. This is used by save, replace, and update
    /// and can be called directly if you want to optimize
    pub fn replace(
        &self,
        store: &mut dyn Storage,
        key: K,
        data: Option<&T>,
        old_data: Option<&T>,
        height: u64,
    ) -> StdResult<()> {
        // this is the key *relative* to the primary map namespace
        let pk = key.joined_key();
        if let Some(old) = old_data {
            for index in self.idx.get_indexes() {
                index.remove(store, &pk, old)?;
            }
        }
        if let Some(updated) = data {
            for index in self.idx.get_indexes() {
                index.save(store, &pk, updated)?;
            }
            self.primary.save(store, key, updated, height)?;
        } else {
            self.primary.remove(store, key, height)?;
        }
        Ok(())
    }

    /// Loads the data, perform the specified action, and store the result
    /// in the database. This is shorthand for some common sequences, which may be useful.
    ///
    /// If the data exists, `action(Some(value))` is called. Otherwise `action(None)` is called.
    pub fn update<A, E>(
        &self,
        store: &mut dyn Storage,
        key: K,
        height: u64,
        action: A,
    ) -> Result<T, E>
    where
        A: FnOnce(Option<T>) -> Result<T, E>,
        E: From<StdError>,
    {
        let input = self.may_load(store, key.clone())?;
        let old_val = input.clone();
        let output = action(input)?;
        self.replace(store, key, Some(&output), old_val.as_ref(), height)?;
        Ok(output)
    }

    // Everything else, that doesn't touch indexers, is just pass-through from self.core,
    // thus can be used from while iterating over indexes

    /// load will return an error if no data is set at the given key, or on parse error
    pub fn load(&self, store: &dyn Storage, key: K) -> StdResult<T> {
        self.primary.load(store, key)
    }

    /// may_load will parse the data stored at the key if present, returns Ok(None) if no data there.
    /// returns an error on issues parsing
    pub fn may_load(&self, store: &dyn Storage, key: K) -> StdResult<Option<T>> {
        self.primary.may_load(store, key)
    }

    // use no_prefix to scan -> range
    pub fn no_prefix_raw(&self) -> Prefix<Vec<u8>, T, K> {
        Prefix::new(self.pk_namespace, &[])
    }
}

// short-cut for simple keys, rather than .prefix(()).range_raw(...)
impl<'a, K, T, I> IndexedSnapshotMap<'a, K, T, I>
where
    K: PrimaryKey<'a> + Prefixer<'a> + KeyDeserialize,
    T: Serialize + DeserializeOwned + Clone,
    I: IndexList<T>,
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
    ) -> Box<dyn Iterator<Item = Vec<u8>> + 'c> {
        self.no_prefix_raw().keys_raw(store, min, max, order)
    }
}

#[cfg(feature = "iterator")]
impl<'a, K, T, I> IndexedSnapshotMap<'a, K, T, I>
where
    T: Serialize + DeserializeOwned + Clone,
    K: PrimaryKey<'a>,
    I: IndexList<T>,
{
    pub fn sub_prefix(&self, p: K::SubPrefix) -> Prefix<K::SuperSuffix, T, K::SuperSuffix> {
        Prefix::new(self.pk_namespace, &p.prefix())
    }

    pub fn prefix(&self, p: K::Prefix) -> Prefix<K::Suffix, T, K::Suffix> {
        Prefix::new(self.pk_namespace, &p.prefix())
    }
}

#[cfg(feature = "iterator")]
impl<'a, K, T, I> IndexedSnapshotMap<'a, K, T, I>
where
    T: Serialize + DeserializeOwned + Clone,
    K: PrimaryKey<'a> + KeyDeserialize,
    I: IndexList<T>,
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
        let mapped = namespaced_prefix_range(store, self.pk_namespace, min, max, order)
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

    fn no_prefix(&self) -> Prefix<K, T, K> {
        Prefix::new(self.pk_namespace, &[])
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use crate::indexes::index_string_tuple;
    use crate::{index_tuple, Index, MultiIndex, UniqueIndex};
    use cosmwasm_std::testing::MockStorage;
    use cosmwasm_std::{MemoryStorage, Order};
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
    struct Data {
        pub name: String,
        pub last_name: String,
        pub age: u32,
    }

    struct DataIndexes<'a> {
        // Last type parameters are for signaling pk deserialization
        pub name: MultiIndex<'a, Vec<u8>, Data, String>,
        pub age: UniqueIndex<'a, u32, Data, String>,
        pub name_lastname: UniqueIndex<'a, (Vec<u8>, Vec<u8>), Data, String>,
    }

    // Future Note: this can likely be macro-derived
    impl<'a> IndexList<Data> for DataIndexes<'a> {
        fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Data>> + '_> {
            let v: Vec<&dyn Index<Data>> = vec![&self.name, &self.age, &self.name_lastname];
            Box::new(v.into_iter())
        }
    }

    // For composite multi index tests
    struct DataCompositeMultiIndex<'a> {
        // Last type parameter is for signaling pk deserialization
        pub name_age: MultiIndex<'a, (Vec<u8>, u32), Data, String>,
    }

    // Future Note: this can likely be macro-derived
    impl<'a> IndexList<Data> for DataCompositeMultiIndex<'a> {
        fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Data>> + '_> {
            let v: Vec<&dyn Index<Data>> = vec![&self.name_age];
            Box::new(v.into_iter())
        }
    }

    // Can we make it easier to define this? (less wordy generic)
    fn build_snapshot_map<'a>() -> IndexedSnapshotMap<'a, &'a str, Data, DataIndexes<'a>> {
        let indexes = DataIndexes {
            name: MultiIndex::new(|d| d.name.as_bytes().to_vec(), "data", "data__name"),
            age: UniqueIndex::new(|d| d.age, "data__age"),
            name_lastname: UniqueIndex::new(
                |d| index_string_tuple(&d.name, &d.last_name),
                "data__name_lastname",
            ),
        };
        IndexedSnapshotMap::new(
            "data",
            "checkpoints",
            "changelog",
            Strategy::EveryBlock,
            indexes,
        )
    }

    fn save_data<'a>(
        store: &mut MockStorage,
        map: &IndexedSnapshotMap<'a, &'a str, Data, DataIndexes<'a>>,
    ) -> (Vec<&'a str>, Vec<Data>) {
        let mut pks = vec![];
        let mut datas = vec![];
        let mut height = 0;
        let data = Data {
            name: "Maria".to_string(),
            last_name: "Doe".to_string(),
            age: 42,
        };
        let pk = "1";
        map.save(store, pk, &data, height).unwrap();
        height += 1;
        pks.push(pk);
        datas.push(data);

        // same name (multi-index), different last name, different age => ok
        let data = Data {
            name: "Maria".to_string(),
            last_name: "Williams".to_string(),
            age: 23,
        };
        let pk = "2";
        map.save(store, pk, &data, height).unwrap();
        height += 1;
        pks.push(pk);
        datas.push(data);

        // different name, different last name, different age => ok
        let data = Data {
            name: "John".to_string(),
            last_name: "Wayne".to_string(),
            age: 32,
        };
        let pk = "3";
        map.save(store, pk, &data, height).unwrap();
        height += 1;
        pks.push(pk);
        datas.push(data);

        let data = Data {
            name: "Maria Luisa".to_string(),
            last_name: "Rodriguez".to_string(),
            age: 12,
        };
        let pk = "4";
        map.save(store, pk, &data, height).unwrap();
        pks.push(pk);
        datas.push(data);

        (pks, datas)
    }

    #[test]
    fn store_and_load_by_index() {
        let mut store = MockStorage::new();
        let map = build_snapshot_map();

        // save data
        let (pks, datas) = save_data(&mut store, &map);
        let pk = pks[0];
        let data = &datas[0];

        // load it properly
        let loaded = map.load(&store, pk).unwrap();
        assert_eq!(*data, loaded);

        let count = map
            .idx
            .name
            .prefix(b"Maria".to_vec())
            .range_raw(&store, None, None, Order::Ascending)
            .count();
        assert_eq!(2, count);

        // load it by secondary index
        let marias: Vec<_> = map
            .idx
            .name
            .prefix(b"Maria".to_vec())
            .range_raw(&store, None, None, Order::Ascending)
            .collect::<StdResult<_>>()
            .unwrap();
        assert_eq!(2, marias.len());
        let (k, v) = &marias[0];
        assert_eq!(pk.as_bytes(), k);
        assert_eq!(data, v);

        // other index doesn't match (1 byte after)
        let count = map
            .idx
            .name
            .prefix(b"Marib".to_vec())
            .range_raw(&store, None, None, Order::Ascending)
            .count();
        assert_eq!(0, count);

        // other index doesn't match (1 byte before)
        let count = map
            .idx
            .name
            .prefix(b"Mari`".to_vec())
            .range_raw(&store, None, None, Order::Ascending)
            .count();
        assert_eq!(0, count);

        // other index doesn't match (longer)
        let count = map
            .idx
            .name
            .prefix(b"Maria5".to_vec())
            .range_raw(&store, None, None, Order::Ascending)
            .count();
        assert_eq!(0, count);

        // match on proper age
        let proper = 42u32;
        let aged = map.idx.age.item(&store, proper).unwrap().unwrap();
        assert_eq!(pk.as_bytes(), aged.0);
        assert_eq!(*data, aged.1);

        // no match on wrong age
        let too_old = 43u32;
        let aged = map.idx.age.item(&store, too_old).unwrap();
        assert_eq!(None, aged);
    }

    #[test]
    fn range_raw_simple_key_by_multi_index() {
        let mut store = MockStorage::new();
        let map = build_snapshot_map();
        let mut height = 1;

        // save data
        let data1 = Data {
            name: "Maria".to_string(),
            last_name: "".to_string(),
            age: 42,
        };
        let pk = "5627";
        map.save(&mut store, pk, &data1, height).unwrap();
        height += 1;

        let data2 = Data {
            name: "Juan".to_string(),
            last_name: "Perez".to_string(),
            age: 13,
        };
        let pk = "5628";
        map.save(&mut store, pk, &data2, height).unwrap();
        height += 1;

        let data3 = Data {
            name: "Maria".to_string(),
            last_name: "Williams".to_string(),
            age: 24,
        };
        let pk = "5629";
        map.save(&mut store, pk, &data3, height).unwrap();
        height += 1;

        let data4 = Data {
            name: "Maria Luisa".to_string(),
            last_name: "Bemberg".to_string(),
            age: 12,
        };
        let pk = "5630";
        map.save(&mut store, pk, &data4, height).unwrap();

        let marias: Vec<_> = map
            .idx
            .name
            .prefix(b"Maria".to_vec())
            .range_raw(&store, None, None, Order::Descending)
            .collect::<StdResult<_>>()
            .unwrap();
        let count = marias.len();
        assert_eq!(2, count);

        // Sorted by (descending) pk
        assert_eq!(marias[0].0, b"5629");
        assert_eq!(marias[1].0, b"5627");
        // Data is correct
        assert_eq!(marias[0].1, data3);
        assert_eq!(marias[1].1, data1);
    }

    #[test]
    fn range_simple_key_by_multi_index() {
        let mut store = MockStorage::new();
        let map = build_snapshot_map();
        let mut height = 1;

        // save data
        let data1 = Data {
            name: "Maria".to_string(),
            last_name: "".to_string(),
            age: 42,
        };
        let pk = "5627";
        map.save(&mut store, pk, &data1, height).unwrap();
        height += 1;

        let data2 = Data {
            name: "Juan".to_string(),
            last_name: "Perez".to_string(),
            age: 13,
        };
        let pk = "5628";
        map.save(&mut store, pk, &data2, height).unwrap();
        height += 1;

        let data3 = Data {
            name: "Maria".to_string(),
            last_name: "Williams".to_string(),
            age: 24,
        };
        let pk = "5629";
        map.save(&mut store, pk, &data3, height).unwrap();
        height += 1;

        let data4 = Data {
            name: "Maria Luisa".to_string(),
            last_name: "Bemberg".to_string(),
            age: 12,
        };
        let pk = "5630";
        map.save(&mut store, pk, &data4, height).unwrap();

        let marias: Vec<_> = map
            .idx
            .name
            .prefix(b"Maria".to_vec())
            .range(&store, None, None, Order::Descending)
            .collect::<StdResult<_>>()
            .unwrap();
        let count = marias.len();
        assert_eq!(2, count);

        // Sorted by (descending) pk
        assert_eq!(marias[0].0, "5629");
        assert_eq!(marias[1].0, "5627");
        // Data is correct
        assert_eq!(marias[0].1, data3);
        assert_eq!(marias[1].1, data1);
    }

    #[test]
    fn changelog_range_works() {
        let mut store = MockStorage::new();
        let map = build_snapshot_map();
        let mut height = 1;

        // simple data for testing
        // EVERY.remove(&mut store, "B", 4).unwrap();
        let data1 = Data {
            name: "Maria".to_string(),
            last_name: "".to_string(),
            age: 42,
        };
        let pk_a = "A";
        map.save(&mut store, pk_a, &data1, height).unwrap();
        height += 1;

        let data2 = Data {
            name: "Juan".to_string(),
            last_name: "Perez".to_string(),
            age: 13,
        };
        let pk_b = "B";
        map.save(&mut store, pk_b, &data2, height).unwrap();
        height += 1;

        let data3 = Data {
            name: "Maria".to_string(),
            last_name: "Williams".to_string(),
            age: 24,
        };
        map.update(&mut store, pk_a, height, |_| -> StdResult<Data> {
            Ok(data3)
        })
        .unwrap();

        height += 1;
        map.remove(&mut store, pk_b, height).unwrap();

        let changes: Vec<_> = map
            .changelog()
            .range(&store, None, None, Order::Ascending)
            .collect::<StdResult<_>>()
            .unwrap();
        let count = changes.len();
        assert_eq!(4, count);

        // sorted by ascending key, height
        assert_eq!(
            changes,
            vec![
                (("A".into(), 1), ChangeSet { old: None }),
                (("A".into(), 3), ChangeSet { old: Some(data1) }),
                (("B".into(), 2), ChangeSet { old: None }),
                (("B".into(), 4), ChangeSet { old: Some(data2) })
            ]
        );
    }

    #[test]
    fn range_raw_composite_key_by_multi_index() {
        let mut store = MockStorage::new();
        let mut height = 2;

        let indexes = DataCompositeMultiIndex {
            name_age: MultiIndex::new(|d| index_tuple(&d.name, d.age), "data", "data__name_age"),
        };
        let map =
            IndexedSnapshotMap::new("data", "checks", "changes", Strategy::EveryBlock, indexes);

        // save data
        let data1 = Data {
            name: "Maria".to_string(),
            last_name: "".to_string(),
            age: 42,
        };
        let pk1 = "5627";
        map.save(&mut store, pk1, &data1, height).unwrap();
        height += 1;

        let data2 = Data {
            name: "Juan".to_string(),
            last_name: "Perez".to_string(),
            age: 13,
        };
        let pk2 = "5628";
        map.save(&mut store, pk2, &data2, height).unwrap();
        height += 1;

        let data3 = Data {
            name: "Maria".to_string(),
            last_name: "Young".to_string(),
            age: 24,
        };
        let pk3 = "5629";
        map.save(&mut store, pk3, &data3, height).unwrap();
        height += 1;

        let data4 = Data {
            name: "Maria Luisa".to_string(),
            last_name: "Bemberg".to_string(),
            age: 43,
        };
        let pk4 = "5630";
        map.save(&mut store, pk4, &data4, height).unwrap();

        let marias: Vec<_> = map
            .idx
            .name_age
            .sub_prefix(b"Maria".to_vec())
            .range_raw(&store, None, None, Order::Descending)
            .collect::<StdResult<_>>()
            .unwrap();
        let count = marias.len();
        assert_eq!(2, count);

        // Pks (sorted by age descending)
        assert_eq!(pk1.as_bytes(), marias[0].0);
        assert_eq!(pk3.as_bytes(), marias[1].0);

        // Data
        assert_eq!(data1, marias[0].1);
        assert_eq!(data3, marias[1].1);
    }

    #[test]
    fn range_composite_key_by_multi_index() {
        let mut store = MockStorage::new();
        let mut height = 2;

        let indexes = DataCompositeMultiIndex {
            name_age: MultiIndex::new(|d| index_tuple(&d.name, d.age), "data", "data__name_age"),
        };
        let map =
            IndexedSnapshotMap::new("data", "checks", "changes", Strategy::EveryBlock, indexes);

        // save data
        let data1 = Data {
            name: "Maria".to_string(),
            last_name: "".to_string(),
            age: 42,
        };
        let pk1 = "5627";
        map.save(&mut store, pk1, &data1, height).unwrap();
        height += 1;

        let data2 = Data {
            name: "Juan".to_string(),
            last_name: "Perez".to_string(),
            age: 13,
        };
        let pk2 = "5628";
        map.save(&mut store, pk2, &data2, height).unwrap();
        height += 1;

        let data3 = Data {
            name: "Maria".to_string(),
            last_name: "Young".to_string(),
            age: 24,
        };
        let pk3 = "5629";
        map.save(&mut store, pk3, &data3, height).unwrap();
        height += 1;

        let data4 = Data {
            name: "Maria Luisa".to_string(),
            last_name: "Bemberg".to_string(),
            age: 43,
        };
        let pk4 = "5630";
        map.save(&mut store, pk4, &data4, height).unwrap();

        let marias: Vec<_> = map
            .idx
            .name_age
            .sub_prefix(b"Maria".to_vec())
            .range(&store, None, None, Order::Descending)
            .collect::<StdResult<_>>()
            .unwrap();
        let count = marias.len();
        assert_eq!(2, count);

        // Pks (sorted by age descending)
        assert_eq!(pk1.to_string(), marias[0].0);
        assert_eq!(pk3.to_string(), marias[1].0);

        // Data
        assert_eq!(data1, marias[0].1);
        assert_eq!(data3, marias[1].1);
    }

    #[test]
    fn unique_index_enforced() {
        let mut store = MockStorage::new();
        let map = build_snapshot_map();
        let mut height = 3;

        // save data
        let (pks, datas) = save_data(&mut store, &map);

        // different name, different last name, same age => error
        let data5 = Data {
            name: "Marta".to_string(),
            last_name: "Laurens".to_string(),
            age: 42,
        };
        let pk5 = "4";

        // enforce this returns some error
        map.save(&mut store, pk5, &data5, height).unwrap_err();
        height += 1;

        // query by unique key
        // match on proper age
        let age42 = 42u32;
        let (k, v) = map.idx.age.item(&store, age42).unwrap().unwrap();
        assert_eq!(k, pks[0].as_bytes());
        assert_eq!(v.name, datas[0].name);
        assert_eq!(v.age, datas[0].age);

        // match on other age
        let age23 = 23u32;
        let (k, v) = map.idx.age.item(&store, age23).unwrap().unwrap();
        assert_eq!(k, pks[1].as_bytes());
        assert_eq!(v.name, datas[1].name);
        assert_eq!(v.age, datas[1].age);

        // if we delete the first one, we can add the blocked one
        map.remove(&mut store, pks[0], height).unwrap();
        height += 1;
        map.save(&mut store, pk5, &data5, height).unwrap();
        // now 42 is the new owner
        let (k, v) = map.idx.age.item(&store, age42).unwrap().unwrap();
        assert_eq!(k, pk5.as_bytes());
        assert_eq!(v.name, data5.name);
        assert_eq!(v.age, data5.age);
    }

    #[test]
    fn unique_index_enforced_composite_key() {
        let mut store = MockStorage::new();
        let map = build_snapshot_map();
        let height = 4;

        // save data
        save_data(&mut store, &map);

        // same name, same lastname => error
        let data5 = Data {
            name: "Maria".to_string(),
            last_name: "Doe".to_string(),
            age: 24,
        };
        let pk5 = "5";
        // enforce this returns some error
        map.save(&mut store, pk5, &data5, height).unwrap_err();
    }

    #[test]
    fn remove_and_update_reflected_on_indexes() {
        let mut store = MockStorage::new();
        let map = build_snapshot_map();
        let mut height = 5;

        let name_count = |map: &IndexedSnapshotMap<&str, Data, DataIndexes>,
                          store: &MemoryStorage,
                          name: &str|
         -> usize {
            map.idx
                .name
                .prefix(name.as_bytes().to_vec())
                .keys_raw(store, None, None, Order::Ascending)
                .count()
        };

        // save data
        let (pks, _) = save_data(&mut store, &map);

        // find 2 Marias, 1 John, and no Mary
        assert_eq!(name_count(&map, &store, "Maria"), 2);
        assert_eq!(name_count(&map, &store, "John"), 1);
        assert_eq!(name_count(&map, &store, "Maria Luisa"), 1);
        assert_eq!(name_count(&map, &store, "Mary"), 0);

        // remove maria 2
        map.remove(&mut store, pks[1], height).unwrap();
        height += 1;

        // change john to mary
        map.update(&mut store, pks[2], height, |d| -> StdResult<_> {
            let mut x = d.unwrap();
            assert_eq!(&x.name, "John");
            x.name = "Mary".to_string();
            Ok(x)
        })
        .unwrap();

        // find 1 maria, 1 maria luisa, no john, and 1 mary
        assert_eq!(name_count(&map, &store, "Maria"), 1);
        assert_eq!(name_count(&map, &store, "Maria Luisa"), 1);
        assert_eq!(name_count(&map, &store, "John"), 0);
        assert_eq!(name_count(&map, &store, "Mary"), 1);
    }

    #[test]
    fn range_raw_simple_key_by_unique_index() {
        let mut store = MockStorage::new();
        let map = build_snapshot_map();

        // save data
        let (pks, datas) = save_data(&mut store, &map);

        let res: StdResult<Vec<_>> = map
            .idx
            .age
            .range_raw(&store, None, None, Order::Ascending)
            .collect();
        let ages = res.unwrap();

        let count = ages.len();
        assert_eq!(4, count);

        // The pks, sorted by age ascending
        assert_eq!(pks[3].as_bytes(), ages[0].0);
        assert_eq!(pks[1].as_bytes(), ages[1].0);
        assert_eq!(pks[2].as_bytes(), ages[2].0);
        assert_eq!(pks[0].as_bytes(), ages[3].0);

        // The associated data
        assert_eq!(datas[3], ages[0].1);
        assert_eq!(datas[1], ages[1].1);
        assert_eq!(datas[2], ages[2].1);
        assert_eq!(datas[0], ages[3].1);
    }

    #[test]
    fn range_simple_key_by_unique_index() {
        let mut store = MockStorage::new();
        let map = build_snapshot_map();

        // save data
        let (pks, datas) = save_data(&mut store, &map);

        let res: StdResult<Vec<_>> = map
            .idx
            .age
            .range(&store, None, None, Order::Ascending)
            .collect();
        let ages = res.unwrap();

        let count = ages.len();
        assert_eq!(4, count);

        // The pks, sorted by age ascending
        assert_eq!(pks[3], ages[0].0);
        assert_eq!(pks[1], ages[1].0);
        assert_eq!(pks[2], ages[2].0);
        assert_eq!(pks[0], ages[3].0);

        // The associated data
        assert_eq!(datas[3], ages[0].1);
        assert_eq!(datas[1], ages[1].1);
        assert_eq!(datas[2], ages[2].1);
        assert_eq!(datas[0], ages[3].1);
    }

    #[test]
    fn range_raw_composite_key_by_unique_index() {
        let mut store = MockStorage::new();
        let map = build_snapshot_map();

        // save data
        let (pks, datas) = save_data(&mut store, &map);

        let res: StdResult<Vec<_>> = map
            .idx
            .name_lastname
            .prefix(b"Maria".to_vec())
            .range_raw(&store, None, None, Order::Ascending)
            .collect();
        let marias = res.unwrap();

        // Only two people are called "Maria"
        let count = marias.len();
        assert_eq!(2, count);

        // The pks
        assert_eq!(pks[0].as_bytes(), marias[0].0);
        assert_eq!(pks[1].as_bytes(), marias[1].0);

        // The associated data
        assert_eq!(datas[0], marias[0].1);
        assert_eq!(datas[1], marias[1].1);
    }

    #[test]
    fn range_composite_key_by_unique_index() {
        let mut store = MockStorage::new();
        let map = build_snapshot_map();

        // save data
        let (pks, datas) = save_data(&mut store, &map);

        let res: StdResult<Vec<_>> = map
            .idx
            .name_lastname
            .prefix(b"Maria".to_vec())
            .range(&store, None, None, Order::Ascending)
            .collect();
        let marias = res.unwrap();

        // Only two people are called "Maria"
        let count = marias.len();
        assert_eq!(2, count);

        // The pks
        assert_eq!(pks[0], marias[0].0);
        assert_eq!(pks[1], marias[1].0);

        // The associated data
        assert_eq!(datas[0], marias[0].1);
        assert_eq!(datas[1], marias[1].1);
    }

    #[test]
    #[cfg(feature = "iterator")]
    fn range_simple_string_key() {
        let mut store = MockStorage::new();
        let map = build_snapshot_map();

        // save data
        let (pks, datas) = save_data(&mut store, &map);

        // let's try to iterate!
        let all: StdResult<Vec<_>> = map.range(&store, None, None, Order::Ascending).collect();
        let all = all.unwrap();
        assert_eq!(
            all,
            pks.clone()
                .into_iter()
                .map(str::to_string)
                .zip(datas.clone().into_iter())
                .collect::<Vec<_>>()
        );

        // let's try to iterate over a range
        let all: StdResult<Vec<_>> = map
            .range(&store, Some(Bound::inclusive("3")), None, Order::Ascending)
            .collect();
        let all = all.unwrap();
        assert_eq!(
            all,
            pks.into_iter()
                .map(str::to_string)
                .zip(datas.into_iter())
                .rev()
                .take(2)
                .rev()
                .collect::<Vec<_>>()
        );
    }

    #[test]
    #[cfg(feature = "iterator")]
    fn prefix_simple_string_key() {
        let mut store = MockStorage::new();
        let map = build_snapshot_map();

        // save data
        let (pks, datas) = save_data(&mut store, &map);

        // Let's prefix and iterate.
        // This is similar to calling range() directly, but added here for completeness / prefix
        // type checks
        let all: StdResult<Vec<_>> = map
            .prefix(())
            .range(&store, None, None, Order::Ascending)
            .collect();
        let all = all.unwrap();
        assert_eq!(
            all,
            pks.clone()
                .into_iter()
                .map(str::to_string)
                .zip(datas.into_iter())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    #[cfg(feature = "iterator")]
    fn sub_prefix_simple_string_key() {
        let mut store = MockStorage::new();
        let map = build_snapshot_map();

        // save data
        let (pks, datas) = save_data(&mut store, &map);

        // Let's sub-prefix and iterate.
        // This is similar to calling range() directly, but added here for completeness / sub_prefix
        // type checks
        let all: StdResult<Vec<_>> = map
            .sub_prefix(())
            .range(&store, None, None, Order::Ascending)
            .collect();
        let all = all.unwrap();
        assert_eq!(
            all,
            pks.clone()
                .into_iter()
                .map(str::to_string)
                .zip(datas.into_iter())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    #[cfg(feature = "iterator")]
    fn prefix_range_simple_key() {
        let mut store = MockStorage::new();

        let indexes = DataCompositeMultiIndex {
            name_age: MultiIndex::new(|d| index_tuple(&d.name, d.age), "data", "data__name_age"),
        };
        let map =
            IndexedSnapshotMap::new("data", "checks", "changes", Strategy::EveryBlock, indexes);

        // save data
        let data1 = Data {
            name: "Maria".to_string(),
            last_name: "".to_string(),
            age: 42,
        };
        let pk1: (&str, &str) = ("1", "5627");
        map.save(&mut store, pk1, &data1, 1).unwrap();

        let data2 = Data {
            name: "Juan".to_string(),
            last_name: "Perez".to_string(),
            age: 13,
        };
        let pk2: (&str, &str) = ("2", "5628");
        map.save(&mut store, pk2, &data2, 1).unwrap();

        let data3 = Data {
            name: "Maria".to_string(),
            last_name: "Young".to_string(),
            age: 24,
        };
        let pk3: (&str, &str) = ("2", "5629");
        map.save(&mut store, pk3, &data3, 1).unwrap();

        let data4 = Data {
            name: "Maria Luisa".to_string(),
            last_name: "Bemberg".to_string(),
            age: 43,
        };
        let pk4: (&str, &str) = ("3", "5630");
        map.save(&mut store, pk4, &data4, 1).unwrap();

        // let's prefix-range and iterate
        let result: StdResult<Vec<_>> = map
            .prefix_range(
                &store,
                Some(PrefixBound::inclusive("2")),
                None,
                Order::Ascending,
            )
            .collect();
        let result = result.unwrap();
        assert_eq!(
            result,
            [
                (("2".to_string(), "5628".to_string()), data2.clone()),
                (("2".to_string(), "5629".to_string()), data3.clone()),
                (("3".to_string(), "5630".to_string()), data4)
            ]
        );

        // let's try to iterate over a more restrictive prefix-range!
        let result: StdResult<Vec<_>> = map
            .prefix_range(
                &store,
                Some(PrefixBound::inclusive("2")),
                Some(PrefixBound::exclusive("3")),
                Order::Ascending,
            )
            .collect();
        let result = result.unwrap();
        assert_eq!(
            result,
            [
                (("2".to_string(), "5628".to_string()), data2),
                (("2".to_string(), "5629".to_string()), data3),
            ]
        );
    }
}
