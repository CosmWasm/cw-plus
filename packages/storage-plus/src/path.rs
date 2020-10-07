use serde::de::DeserializeOwned;
use serde::Serialize;
use std::marker::PhantomData;

use crate::length_prefixed::namespaces_with_key;
use crate::type_helpers::{may_deserialize, must_deserialize};
use cosmwasm_std::{to_vec, StdResult, Storage};

// TODO: build the final storage key (build_storage_key()) in the constructor, so cheap when we reuse
pub struct Path<'a, T>
where
    T: Serialize + DeserializeOwned,
{
    // these are not prefixed
    namespaces: Vec<&'a [u8]>,
    // TODO: get this cheaper...
    // pub key: &'b [u8],
    key: Vec<u8>,
    // see https://doc.rust-lang.org/std/marker/struct.PhantomData.html#unused-type-parameters for why this is needed
    data: PhantomData<T>,
}

impl<'a, T> Path<'a, T>
where
    T: Serialize + DeserializeOwned,
{
    pub fn new(namespaces: Vec<&'a [u8]>, key: Vec<u8>) -> Self {
        Path {
            namespaces,
            key,
            data: PhantomData,
        }
    }

    /// save will serialize the model and store, returns an error on serialization issues
    pub fn save<S: Storage>(&self, store: &mut S, data: &T) -> StdResult<()> {
        let key = self.build_storage_key();
        store.set(&key, &to_vec(data)?);
        Ok(())
    }

    pub fn remove<S: Storage>(&self, store: &mut S) {
        let key = self.build_storage_key();
        store.remove(&key);
    }

    /// load will return an error if no data is set at the given key, or on parse error
    pub fn load<S: Storage>(&self, store: &S) -> StdResult<T> {
        let key = self.build_storage_key();
        let value = store.get(&key);
        must_deserialize(&value)
    }

    /// may_load will parse the data stored at the key if present, returns Ok(None) if no data there.
    /// returns an error on issues parsing
    pub fn may_load<S: Storage>(&self, store: &S) -> StdResult<Option<T>> {
        let key = self.build_storage_key();
        let value = store.get(&key);
        may_deserialize(&value)
    }

    /// This provides the raw storage key that we use to access a given "bucket key".
    /// Calling this with `key = b""` will give us the pk prefix for range queries
    pub(crate) fn build_storage_key(&self) -> Vec<u8> {
        namespaces_with_key(&self.namespaces, &self.key)
    }
}
