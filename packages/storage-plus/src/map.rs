use serde::de::DeserializeOwned;
use serde::Serialize;
use std::marker::PhantomData;

use crate::keys::{Prefixer, PrimaryKey};
use crate::path::Path;
#[cfg(feature = "iterator")]
use crate::prefix::Prefix;

pub struct Map<'a, K, T> {
    namespaces: &'a [&'a [u8]],
    // see https://doc.rust-lang.org/std/marker/struct.PhantomData.html#unused-type-parameters for why this is needed
    key_type: PhantomData<K>,
    data_type: PhantomData<T>,
}

impl<'a, K, T> Map<'a, K, T> {
    pub const fn new(namespaces: &'a [&'a [u8]]) -> Self {
        Map {
            namespaces,
            data_type: PhantomData,
            key_type: PhantomData,
        }
    }
}

impl<'a, K, T> Map<'a, K, T>
where
    T: Serialize + DeserializeOwned,
    K: PrimaryKey<'a>,
{
    pub fn key(&self, k: K) -> Path<T> {
        Path::new(self.namespaces, &k.key())
    }

    #[cfg(feature = "iterator")]
    pub fn prefix(&self, p: K::Prefix) -> Prefix<T> {
        Prefix::new(self.namespaces, &p.prefix())
    }
}

/// short-cut for simple keys, rather than .prefix(()).range(...)
#[cfg(feature = "iterator")]
impl<'a, T> Map<'a, &'a [u8], T>
where
    T: Serialize + DeserializeOwned,
{
    // I would prefer not to copy code from Prefix, but no other way
    // with lifetimes (create Prefix inside function and return ref = no no)
    pub fn range<'c, S: cosmwasm_std::Storage>(
        &'c self,
        store: &'c S,
        start: Option<&[u8]>,
        end: Option<&[u8]>,
        order: cosmwasm_std::Order,
    ) -> Box<dyn Iterator<Item = cosmwasm_std::StdResult<cosmwasm_std::KV<T>>> + 'c> {
        // but the imports here, so we don't have to feature flag them above
        use crate::length_prefixed::to_length_prefixed_nested;
        use crate::namespace_helpers::range_with_prefix;
        use crate::type_helpers::deserialize_kv;

        let prefix = to_length_prefixed_nested(self.namespaces);
        let mapped = range_with_prefix(store, &prefix, start, end, order).map(deserialize_kv::<T>);
        Box::new(mapped)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use serde::{Deserialize, Serialize};

    use cosmwasm_std::testing::MockStorage;
    #[cfg(feature = "iterator")]
    use cosmwasm_std::{Order, StdResult};

    #[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
    struct Data {
        pub name: String,
        pub age: i32,
    }

    const PEOPLE: Map<&[u8], Data> = Map::new(&[b"people", b"_pk"]);

    const ALLOWANCE: Map<(&[u8], &[u8]), u64> = Map::new(&[b"allow", b"_pk"]);

    #[test]
    fn create_path() {
        let path = PEOPLE.key(b"john");
        let key = path.storage_key;
        // this should be prefixed(people) || prefixed(_pk) || john
        assert_eq!("people".len() + "_pk".len() + "john".len() + 4, key.len());
        assert_eq!(b"people".to_vec().as_slice(), &key[2..8]);
    }

    #[test]
    fn save_and_load() {
        let mut store = MockStorage::new();

        // save and load on one key
        let john = PEOPLE.key(b"john");
        let data = Data {
            name: "John".to_string(),
            age: 32,
        };
        assert_eq!(None, john.may_load(&store).unwrap());
        john.save(&mut store, &data).unwrap();
        assert_eq!(data, john.load(&store).unwrap());

        // nothing on another key
        assert_eq!(None, PEOPLE.key(b"jack").may_load(&store).unwrap());

        // same named path gets the data
        assert_eq!(data, PEOPLE.key(b"john").load(&store).unwrap());

        // removing leaves us empty
        john.remove(&mut store);
        assert_eq!(None, john.may_load(&store).unwrap());
    }

    #[test]
    fn composite_keys() {
        let mut store = MockStorage::new();

        // save and load on a composite key
        let allow = ALLOWANCE.key((b"owner", b"spender"));
        assert_eq!(None, allow.may_load(&store).unwrap());
        allow.save(&mut store, &1234).unwrap();
        assert_eq!(1234, allow.load(&store).unwrap());

        // not under other key
        let different = ALLOWANCE
            .key((b"owners", b"pender"))
            .may_load(&store)
            .unwrap();
        assert_eq!(None, different);

        // matches under a copy
        let same = ALLOWANCE.key((b"owner", b"spender")).load(&store).unwrap();
        assert_eq!(1234, same);
    }

    #[test]
    #[cfg(feature = "iterator")]
    fn range_simple_key() {
        let mut store = MockStorage::new();

        // save and load on two keys
        let data = Data {
            name: "John".to_string(),
            age: 32,
        };
        PEOPLE.key(b"john").save(&mut store, &data).unwrap();

        let data2 = Data {
            name: "Jim".to_string(),
            age: 44,
        };
        PEOPLE.key(b"jim").save(&mut store, &data2).unwrap();

        // let's try to iterate!
        let all: StdResult<Vec<_>> = PEOPLE
            .prefix(())
            .range(&store, None, None, Order::Ascending)
            .collect();
        let all = all.unwrap();
        assert_eq!(2, all.len());
        assert_eq!(
            all,
            vec![(b"jim".to_vec(), data2), (b"john".to_vec(), data)]
        );
    }

    #[test]
    #[cfg(feature = "iterator")]
    fn range_composite_key() {
        let mut store = MockStorage::new();

        // save and load on three keys, one under different owner
        ALLOWANCE
            .key((b"owner", b"spender"))
            .save(&mut store, &1000)
            .unwrap();
        ALLOWANCE
            .key((b"owner", b"spender2"))
            .save(&mut store, &3000)
            .unwrap();
        ALLOWANCE
            .key((b"owner2", b"spender"))
            .save(&mut store, &5000)
            .unwrap();

        // let's try to iterate!
        let all: StdResult<Vec<_>> = ALLOWANCE
            .prefix(b"owner")
            .range(&store, None, None, Order::Ascending)
            .collect();
        let all = all.unwrap();
        assert_eq!(2, all.len());
        assert_eq!(
            all,
            vec![(b"spender".to_vec(), 1000), (b"spender2".to_vec(), 3000)]
        );
    }
}
