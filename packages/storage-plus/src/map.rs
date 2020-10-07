use serde::de::DeserializeOwned;
use serde::Serialize;
use std::marker::PhantomData;

use crate::path::Path;
#[cfg(feature = "iterator")]
use crate::prefix::Prefix;

// TODO: where to add PREFIX_PK???? Only in an Indexed Map?

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
    pub fn key(&self, k: K) -> Path<'a, T> {
        let (namespaces, key) = k.namespaced_key(self.namespaces);
        Path::new(namespaces, key)
    }

    #[cfg(feature = "iterator")]
    pub fn prefix(&self, p: K::Prefix) -> Prefix<'a, T> {
        let namespaces = p.namespace(self.namespaces);
        Prefix::new(namespaces)
    }
}

// TODO: move these types and traits into a separate file???

pub trait PrimaryKey<'a> {
    type Prefix: Prefixer<'a>;

    // TODO: get this cheaper...
    // fn namespaced(&self, namespaces: &'a[&'a[u8]]) -> (Vec<&'a[u8]>, &'a [u8]);
    fn namespaced_key(&self, namespaces: &'a [&'a [u8]]) -> (Vec<&'a [u8]>, Vec<u8>);
}

impl<'a> PrimaryKey<'a> for &'a [u8] {
    type Prefix = ();

    fn namespaced_key(&self, namespaces: &'a [&'a [u8]]) -> (Vec<&'a [u8]>, Vec<u8>) {
        // this is simple, we don't add more prefixes
        (namespaces.to_vec(), self.to_vec())
    }
}

impl<'a> PrimaryKey<'a> for (&'a [u8], &'a [u8]) {
    type Prefix = &'a [u8];

    fn namespaced_key(&self, namespaces: &'a [&'a [u8]]) -> (Vec<&'a [u8]>, Vec<u8>) {
        let mut spaces = namespaces.to_vec();
        spaces.push(self.0);
        // move the first part into the namespace, second part as key
        (spaces, self.1.to_vec())
    }
}

impl<'a> PrimaryKey<'a> for (&'a [u8], &'a [u8], &'a [u8]) {
    type Prefix = (&'a [u8], &'a [u8]);

    fn namespaced_key(&self, namespaces: &'a [&'a [u8]]) -> (Vec<&'a [u8]>, Vec<u8>) {
        let mut spaces = namespaces.to_vec();
        spaces.extend_from_slice(&[self.0, self.1]);
        // move the first parts into the namespace, last as key
        (spaces, self.2.to_vec())
    }
}

pub trait Prefixer<'a> {
    fn namespace(&self, namespaces: &'a [&'a [u8]]) -> Vec<&'a [u8]>;
}

impl<'a> Prefixer<'a> for () {
    fn namespace(&self, namespaces: &'a [&'a [u8]]) -> Vec<&'a [u8]> {
        namespaces.to_vec()
    }
}

impl<'a> Prefixer<'a> for &'a [u8] {
    fn namespace(&self, namespaces: &'a [&'a [u8]]) -> Vec<&'a [u8]> {
        let mut spaces = namespaces.to_vec();
        spaces.push(self);
        spaces
    }
}

impl<'a> Prefixer<'a> for (&'a [u8], &'a [u8]) {
    fn namespace(&self, namespaces: &'a [&'a [u8]]) -> Vec<&'a [u8]> {
        let mut spaces = namespaces.to_vec();
        spaces.extend_from_slice(&[self.0, self.1]);
        spaces
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
        let key = path.build_storage_key();
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
