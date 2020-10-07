use serde::de::DeserializeOwned;
use serde::Serialize;
use std::marker::PhantomData;

use crate::path::Path;

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
        let (namespaces, key) = k.namespaced(self.namespaces);
        Path::new(namespaces, key)
    }
}

pub trait PrimaryKey<'a> {
    // TODO: get this cheaper...
    // fn namespaced(&self, namespaces: &'a[&'a[u8]]) -> (Vec<&'a[u8]>, &'a [u8]);
    fn namespaced(&self, namespaces: &'a [&'a [u8]]) -> (Vec<&'a [u8]>, Vec<u8>);
}

impl<'a> PrimaryKey<'a> for &'a [u8] {
    fn namespaced(&self, namespaces: &'a [&'a [u8]]) -> (Vec<&'a [u8]>, Vec<u8>) {
        // this is simple, we don't add more prefixes
        (namespaces.to_vec(), self.to_vec())
    }
}

impl<'a> PrimaryKey<'a> for (&'a [u8], &'a [u8]) {
    fn namespaced(&self, namespaces: &'a [&'a [u8]]) -> (Vec<&'a [u8]>, Vec<u8>) {
        let mut spaces = namespaces.to_vec();
        spaces.push(self.0);
        // move the first part into the namespace, second part as key
        (spaces, self.1.to_vec())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use cosmwasm_std::testing::MockStorage;
    use serde::{Deserialize, Serialize};

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
}
