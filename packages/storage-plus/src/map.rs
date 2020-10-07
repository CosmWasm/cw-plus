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

impl<'a> PrimaryKey<'a> for &[u8] {
    fn namespaced(&self, namespaces: &'a [&'a [u8]]) -> (Vec<&'a [u8]>, Vec<u8>) {
        // this is simple, we don't add more prefixes
        (namespaces.to_vec(), self.to_vec())
    }
}
