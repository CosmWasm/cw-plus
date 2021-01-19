// this module requires iterator to be useful at all
#![cfg(feature = "iterator")]

use cosmwasm_std::{StdError, StdResult, Storage};
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::indexes::Index;
use crate::keys::{EmptyPrefix, Prefixer, PrimaryKey};
use crate::map::Map;
use crate::prefix::{Bound, Prefix};

pub trait IndexList<T> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<T>> + '_>;
}

/// IndexedBucket works like a bucket but has a secondary index
/// TODO: remove traits here and make this const fn new
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
    /// TODO: remove traits here and make this const fn new
    pub fn new(pk_namespace: &'a str, indexes: I) -> Self {
        IndexedMap {
            pk_namespace: pk_namespace.as_bytes(),
            primary: Map::new(pk_namespace),
            idx: indexes,
        }
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
}

// short-cut for simple keys, rather than .prefix(()).range(...)
impl<'a, K, T, I> IndexedMap<'a, K, T, I>
where
    K: PrimaryKey<'a>,
    T: Serialize + DeserializeOwned + Clone,
    I: IndexList<T>,
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

#[cfg(test)]
mod test {
    use super::*;

    use crate::indexes::{index_string, index_string_tuple, MultiIndex, UniqueIndex};
    use crate::{PkOwned, U32Key};
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
        pub name: MultiIndex<'a, Data>,
        pub age: UniqueIndex<'a, U32Key, Data>,
        pub name_lastname: UniqueIndex<'a, (PkOwned, PkOwned), Data>,
    }

    // Future Note: this can likely be macro-derived
    impl<'a> IndexList<Data> for DataIndexes<'a> {
        fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Data>> + '_> {
            let v: Vec<&dyn Index<Data>> = vec![&self.name, &self.age, &self.name_lastname];
            Box::new(v.into_iter())
        }
    }

    // Can we make it easier to define this? (less wordy generic)
    fn build_map<'a>() -> IndexedMap<'a, &'a [u8], Data, DataIndexes<'a>> {
        let indexes = DataIndexes {
            name: MultiIndex::new(|d| index_string(&d.name), "data", "data__name"),
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
            .all_items(&store, &index_string("Maria"))
            .unwrap()
            .len();
        assert_eq!(2, count);

        // TODO: we load by wrong keys - get full storage key!

        // load it by secondary index (we must know how to compute this)
        // let marias: StdResult<Vec<_>> = map
        let marias = map
            .idx
            .name
            .all_items(&store, &index_string("Maria"))
            .unwrap();
        assert_eq!(2, marias.len());
        let (k, v) = &marias[0];
        assert_eq!(pk, k.as_slice());
        assert_eq!(data, v);

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
    fn unique_index_enforced() {
        let mut store = MockStorage::new();
        let map = build_map();

        // save data
        let (pks, datas) = save_data(&mut store, &map);

        // different name, different last name, same age => error
        let data5 = Data {
            name: "Marta".to_string(),
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
                .pks(store, &index_string(name), None, None, Order::Ascending)
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
        let mut map = build_map();

        // save data
        let (pks, datas) = save_data(&mut store, &mut map);

        let res: StdResult<Vec<_>> = map
            .idx
            .age
            .range(&store, None, None, Order::Ascending)
            .collect();
        let ages = res.unwrap();

        let count = ages.len();
        assert_eq!(4, count);

        // The pks, sorted by age ascending
        assert_eq!(pks[3].to_vec(), ages[0].0);
        assert_eq!(pks[1].to_vec(), ages[1].0);
        assert_eq!(pks[2].to_vec(), ages[2].0);
        assert_eq!(pks[0].to_vec(), ages[3].0);

        // The associated data
        assert_eq!(datas[3], ages[0].1);
        assert_eq!(datas[1], ages[1].1);
        assert_eq!(datas[2], ages[2].1);
        assert_eq!(datas[0], ages[3].1);
    }

    #[test]
    fn unique_index_composite_key_range() {
        let mut store = MockStorage::new();
        let mut map = build_map();

        // save data
        let (pks, datas) = save_data(&mut store, &mut map);

        let res: StdResult<Vec<_>> = map
            .idx
            .name_lastname
            .prefix(PkOwned(b"Maria".to_vec()))
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
}
