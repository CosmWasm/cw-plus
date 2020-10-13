// this module requires iterator to be useful at all
#![cfg(feature = "iterator")]

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

use cosmwasm_std::{Binary, Order, StdError, StdResult, Storage, KV};

use crate::map::Map;
use crate::prefix::range_with_prefix;
use crate::{Bound, Endian};

/// MARKER is stored in the multi-index as value, but we only look at the key (which is pk)
const MARKER: u32 = 1;

pub fn index_string(data: &str) -> Vec<u8> {
    data.as_bytes().to_vec()
}

// Look at https://docs.rs/endiannezz/0.4.1/endiannezz/trait.Primitive.html
// if you want to make this generic over all ints
pub fn index_int<T: Endian>(data: T) -> Vec<u8> {
    data.to_be_bytes().into()
}

// 2 main variants:
//  * store (namespace, index_name, idx_value, key) -> b"1" - allows many and references pk
//  * store (namespace, index_name, idx_value) -> {key, value} - allows one and copies pk and data
//  // this would be the primary key - we abstract that too???
//  * store (namespace, index_name, pk) -> value - allows one with data
//
// Note: we cannot store traits with generic functions inside `Box<dyn Index>`,
// so I pull S: Storage to a top-level
pub trait Index<S, T>
where
    S: Storage,
    T: Serialize + DeserializeOwned + Clone,
{
    // TODO: do we make this any Vec<u8> ?
    fn name(&self) -> String;
    fn index(&self, data: &T) -> Vec<u8>;

    // TODO: pk: PrimaryKey not just &[u8] ???
    fn save(&self, store: &mut S, pk: &[u8], data: &T) -> StdResult<()>;
    fn remove(&self, store: &mut S, pk: &[u8], old_data: &T) -> StdResult<()>;

    // these should be implemented by all
    fn pks_by_index<'c>(&self, store: &'c S, idx: &[u8]) -> Box<dyn Iterator<Item = Vec<u8>> + 'c>;

    /// returns all items that match this secondary index, always by pk Ascending
    fn items_by_index<'c>(
        &'c self,
        store: &'c S,
        idx: &[u8],
    ) -> Box<dyn Iterator<Item = StdResult<KV<T>>> + 'c>;

    // TODO: range over secondary index values? (eg. all results with 30 < age < 40)
}

pub struct MultiIndex<'a, S, T>
where
    T: Serialize + DeserializeOwned + Clone,
{
    _name: &'a str,
    idx_fn: fn(&T) -> Vec<u8>,
    idx_map: Map<'a, (&'a [u8], &'a [u8]), u32>,
    // note, we collapse the pubkey - combining everything under the namespace - even if it is composite
    pk_map: Map<'a, &'a [u8], T>,
    typed: PhantomData<S>,
}

impl<'a, S, T> MultiIndex<'a, S, T>
where
    S: Storage,
    T: Serialize + DeserializeOwned + Clone,
{
    // Question: can we do this as a const function??
    // Answer: Only if we remove trait guards and just enforce them with Index implementation
    pub fn new(
        idx_fn: fn(&T) -> Vec<u8>,
        pk_namespace: &'a [u8],
        idx_namespace: &'a [u8],
        name: &'a str,
    ) -> Self {
        MultiIndex {
            _name: name,
            idx_fn,
            pk_map: Map::new(pk_namespace),
            idx_map: Map::new(idx_namespace),
            typed: PhantomData,
        }
    }
}

impl<'a, S, T> Index<S, T> for MultiIndex<'a, S, T>
where
    S: Storage,
    T: Serialize + DeserializeOwned + Clone,
{
    fn name(&self) -> String {
        self._name.to_string()
    }

    fn index(&self, data: &T) -> Vec<u8> {
        (self.idx_fn)(data)
    }

    fn save(&self, store: &mut S, pk: &[u8], data: &T) -> StdResult<()> {
        let idx = self.index(data);
        self.idx_map.save(store, (&idx, &pk), &MARKER)
    }

    fn remove(&self, store: &mut S, pk: &[u8], old_data: &T) -> StdResult<()> {
        let idx = self.index(old_data);
        self.idx_map.remove(store, (&idx, &pk));
        Ok(())
    }

    fn pks_by_index<'c>(&self, store: &'c S, idx: &[u8]) -> Box<dyn Iterator<Item = Vec<u8>> + 'c> {
        let prefix = self.idx_map.prefix(idx);
        let mapped = range_with_prefix(store, &prefix, Bound::None, Bound::None, Order::Ascending)
            .map(|(k, _)| k);
        Box::new(mapped)
    }

    /// returns all items that match this secondary index, always by pk Ascending
    fn items_by_index<'c>(
        &'c self,
        store: &'c S,
        idx: &[u8],
    ) -> Box<dyn Iterator<Item = StdResult<KV<T>>> + 'c> {
        let mapped = self.pks_by_index(store, idx).map(move |pk| {
            let v = self.pk_map.load(store, &pk)?;
            Ok((pk, v))
        });
        Box::new(mapped)
    }
}

#[derive(Deserialize, Serialize, Clone)]
pub(crate) struct UniqueRef<T: Clone> {
    // note, we collapse the pubkey - combining everything under the namespace - even if it is composite
    pk: Binary,
    value: T,
}

pub struct UniqueIndex<'a, S, T>
where
    S: Storage,
    T: Serialize + DeserializeOwned + Clone,
{
    _name: &'a str,
    idx_fn: fn(&T) -> Vec<u8>,
    idx_map: Map<'a, &'a [u8], UniqueRef<T>>,
    typed: PhantomData<S>,
}

impl<'a, S, T> UniqueIndex<'a, S, T>
where
    S: Storage,
    T: Serialize + DeserializeOwned + Clone,
{
    pub fn new(idx_fn: fn(&T) -> Vec<u8>, idx_namespace: &'a [u8], name: &'a str) -> Self {
        UniqueIndex {
            idx_fn,
            idx_map: Map::new(idx_namespace),
            _name: name,
            typed: PhantomData,
        }
    }
}

impl<'a, S, T> Index<S, T> for UniqueIndex<'a, S, T>
where
    S: Storage,
    T: Serialize + DeserializeOwned + Clone,
{
    fn name(&self) -> String {
        self._name.to_string()
    }

    fn index(&self, data: &T) -> Vec<u8> {
        (self.idx_fn)(data)
    }

    fn save(&self, store: &mut S, pk: &[u8], data: &T) -> StdResult<()> {
        let idx = self.index(data);
        // error if this is already set
        self.idx_map
            .update(store, &idx, |existing| -> StdResult<_> {
                match existing {
                    Some(_) => Err(StdError::generic_err(format!(
                        "Violates unique constraint on index `{}`",
                        self._name
                    ))),
                    None => Ok(UniqueRef::<T> {
                        pk: pk.into(),
                        value: data.clone(),
                    }),
                }
            })?;
        Ok(())
    }

    fn remove(&self, store: &mut S, _pk: &[u8], old_data: &T) -> StdResult<()> {
        let idx = self.index(old_data);
        self.idx_map.remove(store, &idx);
        Ok(())
    }

    fn pks_by_index<'c>(&self, store: &'c S, idx: &[u8]) -> Box<dyn Iterator<Item = Vec<u8>> + 'c> {
        let data = match self.idx_map.may_load(store, &idx) {
            Ok(Some(item)) => vec![item.pk.to_vec()],
            Ok(None) => vec![],
            Err(_) => unimplemented!(),
        };
        Box::new(data.into_iter())
    }

    /// returns all items that match this secondary index, always by pk Ascending
    fn items_by_index<'c>(
        &'c self,
        store: &'c S,
        idx: &[u8],
    ) -> Box<dyn Iterator<Item = StdResult<KV<T>>> + 'c> {
        let data = match self.idx_map.may_load(store, &idx) {
            Ok(Some(item)) => vec![Ok((item.pk.to_vec(), item.value))],
            Ok(None) => vec![],
            Err(e) => vec![Err(e)],
        };
        Box::new(data.into_iter())
    }
}
