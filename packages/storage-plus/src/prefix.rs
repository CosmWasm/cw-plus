#![cfg(feature = "iterator")]

use serde::de::DeserializeOwned;
use serde::Serialize;
use std::marker::PhantomData;

use crate::length_prefixed::namespaces_with_key;
use crate::namespace_helpers::range_with_prefix;
use crate::type_helpers::deserialize_kv;
use cosmwasm_std::{Order, StdResult, Storage, KV};

pub struct Prefix<'a, T>
where
    T: Serialize + DeserializeOwned,
{
    // these are not prefixed
    namespaces: Vec<&'a [u8]>,
    // see https://doc.rust-lang.org/std/marker/struct.PhantomData.html#unused-type-parameters for why this is needed
    data: PhantomData<T>,
}

impl<'a, T> Prefix<'a, T>
where
    T: Serialize + DeserializeOwned,
{
    pub fn new(namespaces: Vec<&'a [u8]>) -> Self {
        Prefix {
            namespaces,
            data: PhantomData,
        }
    }

    // TODO: parse out composite key prefix???
    pub fn range<'c, S: Storage>(
        &'c self,
        store: &'c S,
        start: Option<&[u8]>,
        end: Option<&[u8]>,
        order: Order,
    ) -> Box<dyn Iterator<Item = StdResult<KV<T>>> + 'c> {
        let namespace = self.build_storage_prefix();
        let mapped =
            range_with_prefix(store, &namespace, start, end, order).map(deserialize_kv::<T>);
        Box::new(mapped)
    }

    /// This provides the raw storage prefix that we use for ranges
    pub(crate) fn build_storage_prefix(&self) -> Vec<u8> {
        namespaces_with_key(&self.namespaces, b"")
    }
}
