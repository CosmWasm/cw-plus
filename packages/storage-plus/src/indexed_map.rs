// this module requires iterator to be useful at all
#![cfg(feature = "iterator")]

use cosmwasm_std::{StdError, StdResult, Storage, KV};
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::indexes::{Index, MultiIndex};
use crate::keys::{Prefixer, PrimaryKey};
use crate::map::Map;
use crate::prefix::Prefix;

/// reserved name, no index may register
const PREFIX_PK: &[u8] = b"_pk";

/// IndexedBucket works like a bucket but has a secondary index
/// This is a WIP.
/// Step 1 - allow exactly 1 secondary index, no multi-prefix on primary key
/// Step 2 - allow multiple named secondary indexes, no multi-prefix on primary key
/// Step 3 - allow unique indexes - They store {pk: Vec<u8>, value: T} so we don't need to re-lookup
/// Step 4 - allow multiple named secondary indexes, clean composite key support
///
/// Current Status: 2
pub struct IndexedMap<'a, 'x, K, T, S>
where
    'a: 'x,
    K: PrimaryKey<'a>,
    T: Serialize + DeserializeOwned + Clone,
    S: Storage,
{
    root: &'a [u8],
    primary: Map<'a, K, T>,
    indexes: Vec<Box<dyn Index<S, T> + 'x>>,
}

impl<'a, 'x, K, T, S> IndexedMap<'a, 'x, K, T, S>
where
    K: PrimaryKey<'a>,
    T: Serialize + DeserializeOwned + Clone + 'x,
    S: Storage + 'x,
{
    // TODO: figure out a better constructor
    pub fn new(namespace: &'a [u8]) -> Self {
        IndexedMap {
            root: namespace,
            primary: Map::new(namespace),
            indexes: vec![],
        }
    }

    /// Usage:
    /// let mut bucket = IndexedBucket::new(&mut storeage, b"foobar")
    ///                     .with_unique_index("name", |x| x.name.clone())?
    ///                     .with_index("age", by_age)?;
    pub fn with_index(mut self, name: &'x str, indexer: fn(&T) -> Vec<u8>) -> StdResult<Self> {
        self.can_add_index(name)?;
        let index: MultiIndex<'x, S, T> = MultiIndex::new(indexer, self.root, name);
        self.indexes.push(Box::new(index));
        Ok(self)
    }

    /// Usage:
    /// let mut bucket = IndexedBucket::new(&mut storeage, b"foobar")
    ///                     .with_unique_index("name", |x| x.name.clone())?
    ///                     .with_index("age", by_age)?;
    // pub fn with_unique_index(
    //     mut self,
    //     name: &'x str,
    //     indexer: fn(&T) -> Vec<u8>,
    // ) -> StdResult<Self> {
    //     self.can_add_index(name)?;
    //     let index = UniqueIndex::new(indexer, name);
    //     self.indexes.push(Box::new(index));
    //     Ok(self)
    // }

    fn can_add_index(&self, name: &str) -> StdResult<()> {
        if name.as_bytes() == PREFIX_PK {
            return Err(StdError::generic_err(
                "Index _pk is reserved for the primary key",
            ));
        }
        match self.get_index(name) {
            Some(_) => Err(StdError::generic_err(format!(
                "Attempt to write index {} 2 times",
                name
            ))),
            None => Ok(()),
        }
    }

    fn get_index(&self, name: &str) -> Option<&dyn Index<S, T>> {
        for existing in self.indexes.iter() {
            if existing.name() == name {
                return Some(existing.as_ref());
            }
        }
        None
    }

    // TODO: try AsRef<PrimaryKey>
    // TODO: revisit map as well

    /// save will serialize the model and store, returns an error on serialization issues.
    /// this must load the old value to update the indexes properly
    /// if you loaded the old value earlier in the same function, use replace to avoid needless db reads
    pub fn save(&mut self, store: &mut S, key: K, data: &T) -> StdResult<()> {
        let old_data = self.may_load(store, key)?;
        self.replace(store, key, Some(data), old_data.as_ref())
    }

    pub fn remove(&mut self, store: &mut S, key: K) -> StdResult<()> {
        let old_data = self.may_load(store, key)?;
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
        let pk = self.primary.key(key);
        if let Some(old) = old_data {
            for index in self.indexes.iter() {
                index.remove(store, &pk, old)?;
            }
        }
        if let Some(updated) = data {
            for index in self.indexes.iter() {
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
        let input = self.may_load(store, key)?;
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
        Prefix::new(self.root, &p.prefix())
    }

    // /// iterates over the items in pk order
    // pub fn range<'c, S: Storage>(
    //     &'c self,
    //     store: &'c S,
    //     start: Option<&[u8]>,
    //     end: Option<&[u8]>,
    //     order: Order,
    // ) -> Box<dyn Iterator<Item = StdResult<KV<T>>> + 'c> {
    //     self.primary.range(start, end, order)
    // }

    /// returns all pks that where stored under this secondary index, always Ascending
    /// this is mainly an internal function, but can be used direcly if you just want to list ids cheaply
    pub fn pks_by_index<'c>(
        &'c self,
        store: &'c S,
        index_name: &str,
        idx: &[u8],
    ) -> StdResult<Box<dyn Iterator<Item = Vec<u8>> + 'c>> {
        let index = self
            .get_index(index_name)
            .ok_or_else(|| StdError::not_found(index_name))?;
        Ok(index.pks_by_index(&store, idx))
    }

    /// returns all items that match this secondary index, always by pk Ascending
    pub fn items_by_index<'c>(
        &'c self,
        store: &'c S,
        index_name: &str,
        idx: &[u8],
    ) -> StdResult<Box<dyn Iterator<Item = StdResult<KV<T>>> + 'c>> {
        let index = self
            .get_index(index_name)
            .ok_or_else(|| StdError::not_found(index_name))?;
        Ok(index.items_by_index(&store, idx))
    }

    // // this will return None for 0 items, Some(x) for 1 item,
    // // and an error for > 1 item. Only meant to be called on unique
    // // indexes that can return 0 or 1 item
    // pub fn load_unique_index(&self, index_name: &str, idx: &[u8]) -> StdResult<Option<KV<T>>> {
    //     let mut it = self.items_by_index(index_name, idx)?;
    //     let first = it.next().transpose()?;
    //     match first {
    //         None => Ok(None),
    //         Some(one) => match it.next() {
    //             None => Ok(Some(one)),
    //             Some(_) => Err(StdError::generic_err("Unique Index returned 2 matches")),
    //         },
    //     }
    // }
}

#[cfg(test)]
mod test {
    use super::*;

    use crate::indexes::{index_i32, index_string};
    use cosmwasm_std::testing::MockStorage;
    use cosmwasm_std::MemoryStorage;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
    struct Data {
        pub name: String,
        pub age: i32,
    }

    fn build_bucket() -> IndexedMap<'static, 'static, &'static [u8], Data, MemoryStorage> {
        IndexedMap::<&[u8], Data, MemoryStorage>::new(b"data")
            .with_index("name", |d| index_string(&d.name))
            .unwrap()
            // .with_unique_index("age", |d| index_i32(d.age))
            .with_index("age", |d| index_i32(d.age))
            .unwrap()
    }

    #[test]
    fn store_and_load_by_index() {
        let mut store = MockStorage::new();
        let mut map = build_bucket();

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

        // load it by secondary index (we must know how to compute this)
        let marias: StdResult<Vec<_>> = map
            .items_by_index(&store, "name", &index_string("Maria"))
            .unwrap()
            .collect();
        let marias = marias.unwrap();
        assert_eq!(1, marias.len());
        let (k, v) = &marias[0];
        assert_eq!(pk, k.as_slice());
        assert_eq!(&data, v);

        // other index doesn't match (1 byte after)
        let marias: StdResult<Vec<_>> = map
            .items_by_index(&store, "name", &index_string("Marib"))
            .unwrap()
            .collect();
        assert_eq!(0, marias.unwrap().len());

        // other index doesn't match (1 byte before)
        let marias: StdResult<Vec<_>> = map
            .items_by_index(&store, "name", &index_string("Mari`"))
            .unwrap()
            .collect();
        assert_eq!(0, marias.unwrap().len());

        // other index doesn't match (longer)
        let marias: StdResult<Vec<_>> = map
            .items_by_index(&store, "name", &index_string("Maria5"))
            .unwrap()
            .collect();
        assert_eq!(0, marias.unwrap().len());

        // match on proper age
        let proper = index_i32(42);
        let marias: StdResult<Vec<_>> = map
            .items_by_index(&store, "age", &proper)
            .unwrap()
            .collect();
        let marias = marias.unwrap();
        assert_eq!(1, marias.len());

        // no match on wrong age
        let too_old = index_i32(43);
        let marias: StdResult<Vec<_>> = map
            .items_by_index(&store, "age", &too_old)
            .unwrap()
            .collect();
        assert_eq!(0, marias.unwrap().len());
    }

    // #[test]
    // fn unique_index_enforced() {
    //     let mut store = MockStorage::new();
    //     let mut bucket = build_bucket(&mut store);
    //
    //     // first data
    //     let data1 = Data {
    //         name: "Maria".to_string(),
    //         age: 42,
    //     };
    //     let pk1: &[u8] = b"5627";
    //     bucket.save(pk1, &data1).unwrap();
    //
    //     // same name (multi-index), different age => ok
    //     let data2 = Data {
    //         name: "Maria".to_string(),
    //         age: 23,
    //     };
    //     let pk2: &[u8] = b"7326";
    //     bucket.save(pk2, &data2).unwrap();
    //
    //     // different name, same age => error
    //     let data3 = Data {
    //         name: "Marta".to_string(),
    //         age: 42,
    //     };
    //     let pk3: &[u8] = b"8263";
    //     // enforce this returns some error
    //     bucket.save(pk3, &data3).unwrap_err();
    //
    //     // query by unique key
    //     // match on proper age
    //     let age42 = index_i32(42);
    //     let (k, v) = bucket.load_unique_index("age", &age42).unwrap().unwrap();
    //     assert_eq!(k.as_slice(), pk1);
    //     assert_eq!(&v.name, "Maria");
    //     assert_eq!(v.age, 42);
    //
    //     // match on other age
    //     let age23 = index_i32(23);
    //     let (k, v) = bucket.load_unique_index("age", &age23).unwrap().unwrap();
    //     assert_eq!(k.as_slice(), pk2);
    //     assert_eq!(&v.name, "Maria");
    //     assert_eq!(v.age, 23);
    //
    //     // if we delete the first one, we can add the blocked one
    //     bucket.remove(pk1).unwrap();
    //     bucket.save(pk3, &data3).unwrap();
    //     // now 42 is the new owner
    //     let (k, v) = bucket.load_unique_index("age", &age42).unwrap().unwrap();
    //     assert_eq!(k.as_slice(), pk3);
    //     assert_eq!(&v.name, "Marta");
    //     assert_eq!(v.age, 42);
    // }

    #[test]
    fn remove_and_update_reflected_on_indexes() {
        let mut store = MockStorage::new();
        let mut map = build_bucket();

        let name_count =
            |map: &IndexedMap<&[u8], Data, MemoryStorage>, store: &MemoryStorage, name: &str| -> usize {
                map.items_by_index(store, "name", &index_string(name))
                    .unwrap()
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
