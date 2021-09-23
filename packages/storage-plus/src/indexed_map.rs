// this module requires iterator to be useful at all
#![cfg(feature = "iterator")]

use cosmwasm_std::{StdError, StdResult, Storage};
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::indexes::Index;
use crate::iter_helpers::deserialize_v;
use crate::keys::{Prefixer, PrimaryKey};
use crate::map::Map;
use crate::prefix::{namespaced_prefix_range, Bound, Prefix, PrefixBound};
use crate::Path;

pub trait IndexList<T> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<T>> + '_>;
}

// TODO: remove traits here and make this const fn new
/// IndexedBucket works like a bucket but has a secondary index
pub struct IndexedMap<'a, K, T, I>
where
    K: PrimaryKey<'a>,
    T: Serialize + DeserializeOwned + Clone,
    I: IndexList<T>,
{
    pk_namespace: &'a [u8],
    primary: Map<'a, K, T>,
    /// This is meant to be read directly to get the proper types, like:
    /// map.idx.owner.items(...)
    pub idx: I,
}

impl<'a, K, T, I> IndexedMap<'a, K, T, I>
where
    K: PrimaryKey<'a>,
    T: Serialize + DeserializeOwned + Clone,
    I: IndexList<T>,
{
    // TODO: remove traits here and make this const fn new
    pub fn new(pk_namespace: &'a str, indexes: I) -> Self {
        IndexedMap {
            pk_namespace: pk_namespace.as_bytes(),
            primary: Map::new(pk_namespace),
            idx: indexes,
        }
    }

    pub fn key(&self, k: K) -> Path<T> {
        self.primary.key(k)
    }
}

impl<'a, K, T, I> IndexedMap<'a, K, T, I>
where
    K: PrimaryKey<'a>,
    T: Serialize + DeserializeOwned + Clone,
    I: IndexList<T>,
{
    /// save will serialize the model and store, returns an error on serialization issues.
    /// this must load the old value to update the indexes properly
    /// if you loaded the old value earlier in the same function, use replace to avoid needless db reads
    pub fn save(&self, store: &mut dyn Storage, key: K, data: &T) -> StdResult<()> {
        let old_data = self.may_load(store, key.clone())?;
        self.replace(store, key, Some(data), old_data.as_ref())
    }

    pub fn remove(&self, store: &mut dyn Storage, key: K) -> StdResult<()> {
        let old_data = self.may_load(store, key.clone())?;
        self.replace(store, key, None, old_data.as_ref())
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
            self.primary.save(store, key, updated)?;
        } else {
            self.primary.remove(store, key);
        }
        Ok(())
    }

    /// Loads the data, perform the specified action, and store the result
    /// in the database. This is shorthand for some common sequences, which may be useful.
    ///
    /// If the data exists, `action(Some(value))` is called. Otherwise `action(None)` is called.
    pub fn update<A, E>(&self, store: &mut dyn Storage, key: K, action: A) -> Result<T, E>
    where
        A: FnOnce(Option<T>) -> Result<T, E>,
        E: From<StdError>,
    {
        let input = self.may_load(store, key.clone())?;
        let old_val = input.clone();
        let output = action(input)?;
        self.replace(store, key, Some(&output), old_val.as_ref())?;
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

    // use prefix to scan -> range
    pub fn prefix(&self, p: K::Prefix) -> Prefix<T> {
        Prefix::new(self.pk_namespace, &p.prefix())
    }

    // use sub_prefix to scan -> range
    pub fn sub_prefix(&self, p: K::SubPrefix) -> Prefix<T> {
        Prefix::new(self.pk_namespace, &p.prefix())
    }

    // use no_prefix to scan -> range
    fn no_prefix(&self) -> Prefix<T> {
        Prefix::new(self.pk_namespace, &[])
    }
}

// short-cut for simple keys, rather than .prefix(()).range(...)
impl<'a, K, T, I> IndexedMap<'a, K, T, I>
where
    K: PrimaryKey<'a>,
    T: Serialize + DeserializeOwned + Clone,
    I: IndexList<T>,
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
        self.no_prefix().range(store, min, max, order)
    }
}

#[cfg(feature = "iterator")]
impl<'a, K, T, I> IndexedMap<'a, K, T, I>
where
    K: PrimaryKey<'a>,
    T: Serialize + DeserializeOwned + Clone,
    I: IndexList<T>,
{
    /// while range assumes you set the prefix to one element and call range over the last one,
    /// prefix_range accepts bounds for the lowest and highest elements of the Prefix we wish to
    /// accept, and iterates over those. There are some issues that distinguish these to and blindly
    /// casting to Vec<u8> doesn't solve them.
    pub fn prefix_range<'c>(
        &self,
        store: &'c dyn Storage,
        min: Option<PrefixBound<'a, K::Prefix>>,
        max: Option<PrefixBound<'a, K::Prefix>>,
        order: cosmwasm_std::Order,
    ) -> Box<dyn Iterator<Item = StdResult<cosmwasm_std::Pair<T>>> + 'c>
    where
        T: 'c,
        'a: 'c,
    {
        let mapped =
            namespaced_prefix_range(store, self.pk_namespace, min, max, order).map(deserialize_v);
        Box::new(mapped)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use crate::indexes::{index_string_tuple, index_triple, MultiIndex, UniqueIndex};
    use crate::U32Key;
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
        // Second arg is for storing pk
        pub name: MultiIndex<'a, (Vec<u8>, Vec<u8>), Data>,
        pub age: UniqueIndex<'a, U32Key, Data>,
        pub name_lastname: UniqueIndex<'a, (Vec<u8>, Vec<u8>), Data>,
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
        // Third arg needed for storing pk
        pub name_age: MultiIndex<'a, (Vec<u8>, U32Key, Vec<u8>), Data>,
    }

    // Future Note: this can likely be macro-derived
    impl<'a> IndexList<Data> for DataCompositeMultiIndex<'a> {
        fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Data>> + '_> {
            let v: Vec<&dyn Index<Data>> = vec![&self.name_age];
            Box::new(v.into_iter())
        }
    }

    // Can we make it easier to define this? (less wordy generic)
    fn build_map<'a>() -> IndexedMap<'a, &'a [u8], Data, DataIndexes<'a>> {
        let indexes = DataIndexes {
            name: MultiIndex::new(|d, k| (d.name.as_bytes().to_vec(), k), "data", "data__name"),
            age: UniqueIndex::new(|d| U32Key::new(d.age), "data__age"),
            name_lastname: UniqueIndex::new(
                |d| index_string_tuple(&d.name, &d.last_name),
                "data__name_lastname",
            ),
        };
        IndexedMap::new("data", indexes)
    }

    fn save_data<'a>(
        store: &mut MockStorage,
        map: &IndexedMap<'a, &'a [u8], Data, DataIndexes<'a>>,
    ) -> (Vec<&'a [u8]>, Vec<Data>) {
        let mut pks = vec![];
        let mut datas = vec![];
        let data = Data {
            name: "Maria".to_string(),
            last_name: "Doe".to_string(),
            age: 42,
        };
        let pk: &[u8] = b"1";
        map.save(store, pk, &data).unwrap();
        pks.push(pk);
        datas.push(data);

        // same name (multi-index), different last name, different age => ok
        let data = Data {
            name: "Maria".to_string(),
            last_name: "Williams".to_string(),
            age: 23,
        };
        let pk: &[u8] = b"2";
        map.save(store, pk, &data).unwrap();
        pks.push(pk);
        datas.push(data);

        // different name, different last name, different age => ok
        let data = Data {
            name: "John".to_string(),
            last_name: "Wayne".to_string(),
            age: 32,
        };
        let pk: &[u8] = b"3";
        map.save(store, pk, &data).unwrap();
        pks.push(pk);
        datas.push(data);

        let data = Data {
            name: "Maria Luisa".to_string(),
            last_name: "Rodriguez".to_string(),
            age: 12,
        };
        let pk: &[u8] = b"4";
        map.save(store, pk, &data).unwrap();
        pks.push(pk);
        datas.push(data);

        let data = Data {
            name: "Marta".to_string(),
            last_name: "After".to_string(),
            age: 90,
        };
        let pk: &[u8] = b"5";
        map.save(store, pk, &data).unwrap();
        pks.push(pk);
        datas.push(data);

        (pks, datas)
    }

    #[test]
    fn store_and_load_by_index() {
        let mut store = MockStorage::new();
        let map = build_map();

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
            .range(&store, None, None, Order::Ascending)
            .count();
        assert_eq!(2, count);

        // TODO: we load by wrong keys - get full storage key!

        // load it by secondary index (we must know how to compute this)
        // let marias: Vec<_>> = map
        let marias: Vec<_> = map
            .idx
            .name
            .prefix(b"Maria".to_vec())
            .range(&store, None, None, Order::Ascending)
            .collect::<StdResult<_>>()
            .unwrap();
        assert_eq!(2, marias.len());
        let (k, v) = &marias[0];
        assert_eq!(pk, k.as_slice());
        assert_eq!(data, v);

        // other index doesn't match (1 byte after)
        let count = map
            .idx
            .name
            .prefix(b"Marib".to_vec())
            .range(&store, None, None, Order::Ascending)
            .count();
        assert_eq!(0, count);

        // other index doesn't match (1 byte before)
        let count = map
            .idx
            .name
            .prefix(b"Mari`".to_vec())
            .range(&store, None, None, Order::Ascending)
            .count();
        assert_eq!(0, count);

        // other index doesn't match (longer)
        let count = map
            .idx
            .name
            .prefix(b"Maria5".to_vec())
            .range(&store, None, None, Order::Ascending)
            .count();
        assert_eq!(0, count);

        // index_key() over MultiIndex works (empty pk)
        // In a MultiIndex, an index key is composed by the index and the primary key.
        // Primary key may be empty (so that to iterate over all elements that match just the index)
        let key = (b"Maria".to_vec(), b"".to_vec());
        // Use the index_key() helper to build the (raw) index key
        let key = map.idx.name.index_key(key);
        // Iterate using a bound over the raw key
        let count = map
            .idx
            .name
            .range(&store, Some(Bound::inclusive(key)), None, Order::Ascending)
            .count();
        // gets from the first "Maria" until the end
        assert_eq!(4, count);

        // index_key() over MultiIndex works (non-empty pk)
        // Build key including a non-empty pk
        let key = (b"Maria".to_vec(), b"1".to_vec());
        // Use the index_key() helper to build the (raw) index key
        let key = map.idx.name.index_key(key);
        // Iterate using a (exclusive) bound over the raw key.
        // (Useful for pagination / continuation contexts).
        let count = map
            .idx
            .name
            .range(&store, Some(Bound::exclusive(key)), None, Order::Ascending)
            .count();
        // gets from the 2nd "Maria" until the end
        assert_eq!(3, count);

        // index_key() over UniqueIndex works.
        let age_key = U32Key::from(23);
        // Use the index_key() helper to build the (raw) index key
        let age_key = map.idx.age.index_key(age_key);
        // Iterate using a (inclusive) bound over the raw key.
        let count = map
            .idx
            .age
            .range(
                &store,
                Some(Bound::inclusive(age_key)),
                None,
                Order::Ascending,
            )
            .count();
        // gets all the greater than or equal to 23 years old people
        assert_eq!(4, count);

        // match on proper age
        let proper = U32Key::new(42);
        let aged = map.idx.age.item(&store, proper).unwrap().unwrap();
        assert_eq!(pk.to_vec(), aged.0);
        assert_eq!(*data, aged.1);

        // no match on wrong age
        let too_old = U32Key::new(43);
        let aged = map.idx.age.item(&store, too_old).unwrap();
        assert_eq!(None, aged);
    }

    #[test]
    fn range_simple_key_by_multi_index() {
        let mut store = MockStorage::new();
        let map = build_map();

        // save data
        let data1 = Data {
            name: "Maria".to_string(),
            last_name: "".to_string(),
            age: 42,
        };
        let pk: &[u8] = b"5627";
        map.save(&mut store, pk, &data1).unwrap();

        let data2 = Data {
            name: "Juan".to_string(),
            last_name: "Perez".to_string(),
            age: 13,
        };
        let pk: &[u8] = b"5628";
        map.save(&mut store, pk, &data2).unwrap();

        let data3 = Data {
            name: "Maria".to_string(),
            last_name: "Williams".to_string(),
            age: 24,
        };
        let pk: &[u8] = b"5629";
        map.save(&mut store, pk, &data3).unwrap();

        let data4 = Data {
            name: "Maria Luisa".to_string(),
            last_name: "Bemberg".to_string(),
            age: 12,
        };
        let pk: &[u8] = b"5630";
        map.save(&mut store, pk, &data4).unwrap();

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
        assert_eq!(marias[0].0, b"5629");
        assert_eq!(marias[1].0, b"5627");
        // Data is correct
        assert_eq!(marias[0].1, data3);
        assert_eq!(marias[1].1, data1);
    }

    #[test]
    fn range_composite_key_by_multi_index() {
        let mut store = MockStorage::new();

        let indexes = DataCompositeMultiIndex {
            name_age: MultiIndex::new(
                |d, k| index_triple(&d.name, d.age, k),
                "data",
                "data__name_age",
            ),
        };
        let map = IndexedMap::new("data", indexes);

        // save data
        let data1 = Data {
            name: "Maria".to_string(),
            last_name: "".to_string(),
            age: 42,
        };
        let pk1: &[u8] = b"5627";
        map.save(&mut store, pk1, &data1).unwrap();

        let data2 = Data {
            name: "Juan".to_string(),
            last_name: "Perez".to_string(),
            age: 13,
        };
        let pk2: &[u8] = b"5628";
        map.save(&mut store, pk2, &data2).unwrap();

        let data3 = Data {
            name: "Maria".to_string(),
            last_name: "Young".to_string(),
            age: 24,
        };
        let pk3: &[u8] = b"5629";
        map.save(&mut store, pk3, &data3).unwrap();

        let data4 = Data {
            name: "Maria Luisa".to_string(),
            last_name: "Bemberg".to_string(),
            age: 43,
        };
        let pk4: &[u8] = b"5630";
        map.save(&mut store, pk4, &data4).unwrap();

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
        assert_eq!(pk1, marias[0].0);
        assert_eq!(pk3, marias[1].0);

        // Data
        assert_eq!(data1, marias[0].1);
        assert_eq!(data3, marias[1].1);
    }

    #[test]
    fn unique_index_enforced() {
        let mut store = MockStorage::new();
        let map = build_map();

        // save data
        let (pks, datas) = save_data(&mut store, &map);

        // different name, different last name, same age => error
        let data5 = Data {
            name: "Marcel".to_string(),
            last_name: "Laurens".to_string(),
            age: 42,
        };
        let pk5: &[u8] = b"4";

        // enforce this returns some error
        map.save(&mut store, pk5, &data5).unwrap_err();

        // query by unique key
        // match on proper age
        let age42 = U32Key::new(42);
        let (k, v) = map.idx.age.item(&store, age42.clone()).unwrap().unwrap();
        assert_eq!(k.as_slice(), pks[0]);
        assert_eq!(v.name, datas[0].name);
        assert_eq!(v.age, datas[0].age);

        // match on other age
        let age23 = U32Key::new(23);
        let (k, v) = map.idx.age.item(&store, age23).unwrap().unwrap();
        assert_eq!(k.as_slice(), pks[1]);
        assert_eq!(v.name, datas[1].name);
        assert_eq!(v.age, datas[1].age);

        // if we delete the first one, we can add the blocked one
        map.remove(&mut store, pks[0]).unwrap();
        map.save(&mut store, pk5, &data5).unwrap();
        // now 42 is the new owner
        let (k, v) = map.idx.age.item(&store, age42).unwrap().unwrap();
        assert_eq!(k.as_slice(), pk5);
        assert_eq!(v.name, data5.name);
        assert_eq!(v.age, data5.age);
    }

    #[test]
    fn unique_index_enforced_composite_key() {
        let mut store = MockStorage::new();
        let map = build_map();

        // save data
        save_data(&mut store, &map);

        // same name, same lastname => error
        let data5 = Data {
            name: "Maria".to_string(),
            last_name: "Doe".to_string(),
            age: 24,
        };
        let pk5: &[u8] = b"5";
        // enforce this returns some error
        map.save(&mut store, pk5, &data5).unwrap_err();
    }

    #[test]
    fn remove_and_update_reflected_on_indexes() {
        let mut store = MockStorage::new();
        let map = build_map();

        let name_count = |map: &IndexedMap<&[u8], Data, DataIndexes>,
                          store: &MemoryStorage,
                          name: &str|
         -> usize {
            map.idx
                .name
                .prefix(name.as_bytes().to_vec())
                .keys(store, None, None, Order::Ascending)
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
        map.remove(&mut store, pks[1]).unwrap();

        // change john to mary
        map.update(&mut store, pks[2], |d| -> StdResult<_> {
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
    fn unique_index_simple_key_range() {
        let mut store = MockStorage::new();
        let map = build_map();

        // save data
        let (pks, datas) = save_data(&mut store, &map);

        let res: StdResult<Vec<_>> = map
            .idx
            .age
            .range(&store, None, None, Order::Ascending)
            .collect();
        let ages = res.unwrap();

        let count = ages.len();
        assert_eq!(5, count);

        // The pks, sorted by age ascending
        assert_eq!(pks[4].to_vec(), ages[4].0);
        assert_eq!(pks[3].to_vec(), ages[0].0);
        assert_eq!(pks[1].to_vec(), ages[1].0);
        assert_eq!(pks[2].to_vec(), ages[2].0);
        assert_eq!(pks[0].to_vec(), ages[3].0);

        // The associated data
        assert_eq!(datas[4], ages[4].1);
        assert_eq!(datas[3], ages[0].1);
        assert_eq!(datas[1], ages[1].1);
        assert_eq!(datas[2], ages[2].1);
        assert_eq!(datas[0], ages[3].1);
    }

    #[test]
    fn unique_index_composite_key_range() {
        let mut store = MockStorage::new();
        let map = build_map();

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
        assert_eq!(pks[0].to_vec(), marias[0].0);
        assert_eq!(pks[1].to_vec(), marias[1].0);

        // The associated data
        assert_eq!(datas[0], marias[0].1);
        assert_eq!(datas[1], marias[1].1);
    }

    mod inclusive_bound {
        use super::*;
        use crate::U64Key;

        struct Indexes<'a> {
            secondary: MultiIndex<'a, (U64Key, Vec<u8>), u64>,
        }

        impl<'a> IndexList<u64> for Indexes<'a> {
            fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<u64>> + '_> {
                let v: Vec<&dyn Index<u64>> = vec![&self.secondary];
                Box::new(v.into_iter())
            }
        }

        #[test]
        #[cfg(feature = "iterator")]
        fn composite_key_query() {
            let indexes = Indexes {
                secondary: MultiIndex::new(
                    |secondary, k| (U64Key::new(*secondary), k),
                    "test_map",
                    "test_map__secondary",
                ),
            };
            let map = IndexedMap::<&str, u64, Indexes>::new("test_map", indexes);
            let mut store = MockStorage::new();

            map.save(&mut store, "one", &1).unwrap();
            map.save(&mut store, "two", &2).unwrap();

            let items: Vec<_> = map
                .idx
                .secondary
                .prefix_range(
                    &store,
                    None,
                    Some(PrefixBound::inclusive(1)),
                    Order::Ascending,
                )
                .collect::<Result<_, _>>()
                .unwrap();

            // Strip the index from values (for simpler comparison)
            let items: Vec<_> = items.into_iter().map(|(_, v)| v).collect();

            assert_eq!(items, vec![1]);
        }
    }
}
