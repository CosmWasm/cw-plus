// this module requires iterator to be useful at all
#![cfg(feature = "iterator")]

use cosmwasm_std::{StdError, StdResult, Storage};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::marker::PhantomData;

use crate::indexes::Index;
use crate::keys::{EmptyPrefix, Prefixer, PrimaryKey};
use crate::map::Map;
use crate::prefix::{Bound, Prefix};

pub trait IndexList<S, T> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<S, T>> + '_>;
}

/// IndexedBucket works like a bucket but has a secondary index
/// TODO: remove traits here and make this const fn new
pub struct IndexedMap<'a, K, T, S, I>
where
    K: PrimaryKey<'a>,
    T: Serialize + DeserializeOwned + Clone,
    I: IndexList<S, T>,
    S: Storage,
{
    pk_namespace: &'a [u8],
    primary: Map<'a, K, T>,
    /// This is meant to be read directly to get the proper types, like:
    /// map.idx.owner.items(...)
    pub idx: I,
    typed_store: PhantomData<S>,
}

impl<'a, K, T, S, I> IndexedMap<'a, K, T, S, I>
where
    K: PrimaryKey<'a>,
    T: Serialize + DeserializeOwned + Clone,
    S: Storage,
    I: IndexList<S, T>,
{
    /// TODO: remove traits here and make this const fn new
    pub fn new(pk_namespace: &'a [u8], indexes: I) -> Self {
        IndexedMap {
            pk_namespace,
            primary: Map::new(pk_namespace),
            idx: indexes,
            typed_store: PhantomData,
        }
    }
}

impl<'a, K, T, S, I> IndexedMap<'a, K, T, S, I>
where
    K: PrimaryKey<'a>,
    T: Serialize + DeserializeOwned + Clone,
    S: Storage,
    I: IndexList<S, T>,
{
    /// save will serialize the model and store, returns an error on serialization issues.
    /// this must load the old value to update the indexes properly
    /// if you loaded the old value earlier in the same function, use replace to avoid needless db reads
    pub fn save(&mut self, store: &mut S, key: K, data: &T) -> StdResult<()> {
        let old_data = self.may_load(store, key.clone())?;
        self.replace(store, key, Some(data), old_data.as_ref())
    }

    pub fn remove(&mut self, store: &mut S, key: K) -> StdResult<()> {
        let old_data = self.may_load(store, key.clone())?;
        self.replace(store, key, None, old_data.as_ref())
    }

    /// replace writes data to key. old_data must be the current stored value (from a previous load)
    /// and is used to properly update the index. This is used by save, replace, and update
    /// and can be called directly if you want to optimize
    pub fn replace(
        &mut self,
        store: &mut S,
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
    pub fn update<A, E>(&mut self, store: &mut S, key: K, action: A) -> Result<T, E>
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
    pub fn load(&self, store: &S, key: K) -> StdResult<T> {
        self.primary.load(store, key)
    }

    /// may_load will parse the data stored at the key if present, returns Ok(None) if no data there.
    /// returns an error on issues parsing
    pub fn may_load(&self, store: &S, key: K) -> StdResult<Option<T>> {
        self.primary.may_load(store, key)
    }

    // use prefix to scan -> range
    pub fn prefix(&self, p: K::Prefix) -> Prefix<T> {
        Prefix::new(self.pk_namespace, &p.prefix())
    }
}

// short-cut for simple keys, rather than .prefix(()).range(...)
impl<'a, K, T, S, I> IndexedMap<'a, K, T, S, I>
where
    K: PrimaryKey<'a>,
    T: Serialize + DeserializeOwned + Clone,
    S: Storage,
    I: IndexList<S, T>,
    K::Prefix: EmptyPrefix,
{
    // I would prefer not to copy code from Prefix, but no other way
    // with lifetimes (create Prefix inside function and return ref = no no)
    pub fn range<'c>(
        &self,
        store: &'c S,
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

#[cfg(test)]
mod test {
    use super::*;

    use crate::indexes::{index_int, index_string, MultiIndex, UniqueIndex};
    use cosmwasm_std::testing::MockStorage;
    use cosmwasm_std::{MemoryStorage, Order};
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
    struct Data {
        pub name: String,
        pub age: i32,
    }

    struct DataIndexes<'a, S: Storage> {
        pub name: MultiIndex<'a, S, Data>,
        pub age: UniqueIndex<'a, S, Data>,
    }

    // Future Note: this can likely be macro-derived
    impl<'a, S: Storage> IndexList<S, Data> for DataIndexes<'a, S> {
        fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<S, Data>> + '_> {
            let v: Vec<&dyn Index<S, Data>> = vec![&self.name, &self.age];
            Box::new(v.into_iter())
        }
    }

    // Can we make it easier to define this? (less wordy generic)
    fn build_map<'a, S: Storage>() -> IndexedMap<'a, &'a [u8], Data, S, DataIndexes<'a, S>> {
        let indexes = DataIndexes {
            name: MultiIndex::new(|d| index_string(&d.name), b"data", b"data__name"),
            age: UniqueIndex::new(|d| index_int(d.age), b"data__age"),
        };
        IndexedMap::new(b"data", indexes)
    }

    #[test]
    fn store_and_load_by_index() {
        let mut store = MockStorage::new();
        let mut map = build_map();

        // save data
        let data = Data {
            name: "Maria".to_string(),
            age: 42,
        };
        let pk: &[u8] = b"5627";
        map.save(&mut store, pk, &data).unwrap();

        // load it properly
        let loaded = map.load(&store, pk).unwrap();
        assert_eq!(data, loaded);

        let count = map
            .idx
            .name
            .all_items(&store, &index_string("Maria"))
            .unwrap()
            .len();
        assert_eq!(1, count);

        // TODO: we load by wrong keys - get full storage key!

        // load it by secondary index (we must know how to compute this)
        // let marias: StdResult<Vec<_>> = map
        let marias = map
            .idx
            .name
            .all_items(&store, &index_string("Maria"))
            .unwrap();
        assert_eq!(1, marias.len());
        let (k, v) = &marias[0];
        assert_eq!(pk, k.as_slice());
        assert_eq!(&data, v);

        // other index doesn't match (1 byte after)
        let count = map
            .idx
            .name
            .all_items(&store, &index_string("Marib"))
            .unwrap()
            .len();
        assert_eq!(0, count);

        // other index doesn't match (1 byte before)
        let count = map
            .idx
            .name
            .all_items(&store, &index_string("Mari`"))
            .unwrap()
            .len();
        assert_eq!(0, count);

        // other index doesn't match (longer)
        let count = map
            .idx
            .name
            .all_items(&store, &index_string("Maria5"))
            .unwrap()
            .len();
        assert_eq!(0, count);

        // match on proper age
        let proper = index_int(42);
        let aged = map.idx.age.item(&store, &proper).unwrap().unwrap();
        assert_eq!(pk.to_vec(), aged.0);
        assert_eq!(data, aged.1);

        // no match on wrong age
        let too_old = index_int(43);
        let aged = map.idx.age.item(&store, &too_old).unwrap();
        assert_eq!(None, aged);
    }

    #[test]
    fn unique_index_enforced() {
        let mut store = MockStorage::new();
        let mut map = build_map();

        // first data
        let data1 = Data {
            name: "Maria".to_string(),
            age: 42,
        };
        let pk1: &[u8] = b"5627";
        map.save(&mut store, pk1, &data1).unwrap();

        // same name (multi-index), different age => ok
        let data2 = Data {
            name: "Maria".to_string(),
            age: 23,
        };
        let pk2: &[u8] = b"7326";
        map.save(&mut store, pk2, &data2).unwrap();

        // different name, same age => error
        let data3 = Data {
            name: "Marta".to_string(),
            age: 42,
        };
        let pk3: &[u8] = b"8263";
        // enforce this returns some error
        map.save(&mut store, pk3, &data3).unwrap_err();

        // query by unique key
        // match on proper age
        let age42 = index_int(42);
        let (k, v) = map.idx.age.item(&store, &age42).unwrap().unwrap();
        assert_eq!(k.as_slice(), pk1);
        assert_eq!(&v.name, "Maria");
        assert_eq!(v.age, 42);

        // match on other age
        let age23 = index_int(23);
        let (k, v) = map.idx.age.item(&store, &age23).unwrap().unwrap();
        assert_eq!(k.as_slice(), pk2);
        assert_eq!(&v.name, "Maria");
        assert_eq!(v.age, 23);

        // if we delete the first one, we can add the blocked one
        map.remove(&mut store, pk1).unwrap();
        map.save(&mut store, pk3, &data3).unwrap();
        // now 42 is the new owner
        let (k, v) = map.idx.age.item(&store, &age42).unwrap().unwrap();
        assert_eq!(k.as_slice(), pk3);
        assert_eq!(&v.name, "Marta");
        assert_eq!(v.age, 42);
    }

    #[test]
    fn remove_and_update_reflected_on_indexes() {
        let mut store = MockStorage::new();
        let mut map = build_map();

        let name_count =
            |map: &IndexedMap<&[u8], Data, MemoryStorage, DataIndexes<MemoryStorage>>,
             store: &MemoryStorage,
             name: &str|
             -> usize {
                map.idx
                    .name
                    .pks(store, &index_string(name), None, None, Order::Ascending)
                    .count()
            };

        // set up some data
        let data1 = Data {
            name: "John".to_string(),
            age: 22,
        };
        let pk1: &[u8] = b"john";
        map.save(&mut store, pk1, &data1).unwrap();
        let data2 = Data {
            name: "John".to_string(),
            age: 25,
        };
        let pk2: &[u8] = b"john2";
        map.save(&mut store, pk2, &data2).unwrap();
        let data3 = Data {
            name: "Fred".to_string(),
            age: 33,
        };
        let pk3: &[u8] = b"fred";
        map.save(&mut store, pk3, &data3).unwrap();

        // find 2 Johns, 1 Fred, and no Mary
        assert_eq!(name_count(&map, &store, "John"), 2);
        assert_eq!(name_count(&map, &store, "Fred"), 1);
        assert_eq!(name_count(&map, &store, "Mary"), 0);

        // remove john 2
        map.remove(&mut store, pk2).unwrap();
        // change fred to mary
        map.update(&mut store, pk3, |d| -> StdResult<_> {
            let mut x = d.unwrap();
            assert_eq!(&x.name, "Fred");
            x.name = "Mary".to_string();
            Ok(x)
        })
        .unwrap();

        // find 1 Johns, no Fred, and 1 Mary
        assert_eq!(name_count(&map, &store, "John"), 1);
        assert_eq!(name_count(&map, &store, "Fred"), 0);
        assert_eq!(name_count(&map, &store, "Mary"), 1);
    }
}
