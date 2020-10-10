#![cfg(feature = "iterator")]
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::marker::PhantomData;

use crate::helpers::nested_namespaces_with_key;
use crate::iter_helpers::{deserialize_kv, range_with_prefix};
use cosmwasm_std::{Order, StdResult, Storage, KV};

pub struct Prefix<T>
where
    T: Serialize + DeserializeOwned,
{
    /// all namespaces prefixes and concatenated with the key
    pub(crate) storage_prefix: Vec<u8>,
    // see https://doc.rust-lang.org/std/marker/struct.PhantomData.html#unused-type-parameters for why this is needed
    data: PhantomData<T>,
}

impl<T> Prefix<T>
where
    T: Serialize + DeserializeOwned,
{
    pub fn new(top_name: &[u8], sub_names: &[&[u8]]) -> Self {
        // FIXME: we can use a custom function here, probably make this cleaner
        let storage_prefix = nested_namespaces_with_key(&[top_names], sub_names, b"");
        Prefix {
            storage_prefix,
            data: PhantomData,
        }
    }

    // TODO: parse out composite key prefix???
    pub fn range<'a, S: Storage>(
        &'a self,
        store: &'a S,
        start: Option<&[u8]>,
        end: Option<&[u8]>,
        order: Order,
    ) -> Box<dyn Iterator<Item = StdResult<KV<T>>> + 'a> {
        let mapped = range_with_prefix(store, &self.storage_prefix, start, end, order)
            .map(deserialize_kv::<T>);
        Box::new(mapped)
    }
}
