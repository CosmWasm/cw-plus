// this module requires iterator to be useful at all
#![cfg(feature = "iterator")]

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Binary, Order, StdError, StdResult, Storage, KV};

use crate::keys::{EmptyPrefix, Prefixer};
use crate::map::Map;
use crate::prefix::range_with_prefix;
use crate::{Bound, Endian, Prefix, PrimaryKey};
use std::marker::PhantomData;

/// MARKER is stored in the multi-index as value, but we only look at the key (which is pk)
const MARKER: bool = true;

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
pub trait Index<T>
where
    T: Serialize + DeserializeOwned + Clone,
{
    fn save(&self, store: &mut dyn Storage, pk: &[u8], data: &T) -> StdResult<()>;
    fn remove(&self, store: &mut dyn Storage, pk: &[u8], old_data: &T) -> StdResult<()>;
}

pub struct MultiIndex<'a, K, T> {
    index: fn(&T) -> Vec<u8>,
    idx_namespace: &'a [u8],
    idx_map: Map<'a, (&'a [u8], &'a [u8]), bool>,
    // note, we collapse the pk - combining everything under the namespace - even if it is composite
    pk_map: Map<'a, &'a [u8], T>,
    idx_type: PhantomData<K>,
}

impl<'a, K, T> MultiIndex<'a, K, T> {
    // TODO: make this a const fn
    pub fn new(idx_fn: fn(&T) -> Vec<u8>, pk_namespace: &'a str, idx_namespace: &'a str) -> Self {
        MultiIndex {
            index: idx_fn,
            pk_map: Map::new(pk_namespace),
            idx_namespace: idx_namespace.as_bytes(),
            idx_map: Map::new(idx_namespace),
            idx_type: PhantomData,
        }
    }
}

impl<'a, K, T> Index<T> for MultiIndex<'a, K, T>
where
    T: Serialize + DeserializeOwned + Clone,
    K: PrimaryKey<'a>,
{
    fn save(&self, store: &mut dyn Storage, pk: &[u8], data: &T) -> StdResult<()> {
        let idx = (self.index)(data);
        self.idx_map.save(store, (&idx, &pk), &MARKER)
    }

    fn remove(&self, store: &mut dyn Storage, pk: &[u8], old_data: &T) -> StdResult<()> {
        let idx = (self.index)(old_data);
        self.idx_map.remove(store, (&idx, &pk));
        Ok(())
    }
}

impl<'a, K, T> MultiIndex<'a, K, T>
where
    T: Serialize + DeserializeOwned + Clone,
    K: PrimaryKey<'a>,
{
    pub fn pks<'c>(
        &self,
        store: &'c dyn Storage,
        idx: &[u8],
        min: Option<Bound>,
        max: Option<Bound>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Vec<u8>> + 'c> {
        let prefix = self.idx_map.prefix(idx);
        let mapped = range_with_prefix(store, &prefix, min, max, order).map(|(k, _)| k);
        Box::new(mapped)
    }

    /// returns all items that match this secondary index, always by pk Ascending
    pub fn items<'c>(
        &'c self,
        store: &'c dyn Storage,
        idx: &[u8],
        min: Option<Bound>,
        max: Option<Bound>,
        order: Order,
    ) -> Box<dyn Iterator<Item = StdResult<KV<T>>> + 'c> {
        let mapped = self.pks(store, idx, min, max, order).map(move |pk| {
            let v = self.pk_map.load(store, &pk)?;
            Ok((pk, v))
        });
        Box::new(mapped)
    }

    pub fn prefix(&self, p: K::Prefix) -> Prefix<T> {
        Prefix::new(self.idx_namespace, &p.prefix())
    }

    #[cfg(test)]
    pub fn count<'c>(&self, store: &'c dyn Storage, idx: &[u8]) -> usize {
        self.pks(store, idx, None, None, Order::Ascending).count()
    }

    #[cfg(test)]
    pub fn all_pks<'c>(&self, store: &'c dyn Storage, idx: &[u8]) -> Vec<Vec<u8>> {
        self.pks(store, idx, None, None, Order::Ascending).collect()
    }

    #[cfg(test)]
    pub fn all_items<'c>(&self, store: &'c dyn Storage, idx: &[u8]) -> StdResult<Vec<KV<T>>> {
        self.items(store, idx, None, None, Order::Ascending)
            .collect()
    }
}

// short-cut for simple keys, rather than .prefix(()).range(...)
impl<'a, K, T> MultiIndex<'a, K, T>
where
    T: Serialize + DeserializeOwned + Clone,
    K: PrimaryKey<'a>,
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

#[derive(Deserialize, Serialize)]
pub(crate) struct UniqueRef<T> {
    // note, we collapse the pk - combining everything under the namespace - even if it is composite
    pk: Binary,
    value: T,
}

pub struct UniqueIndex<'a, T> {
    index: fn(&T) -> Vec<u8>,
    idx_map: Map<'a, &'a [u8], UniqueRef<T>>,
}

impl<'a, T> UniqueIndex<'a, T> {
    // TODO: make this a const fn
    pub fn new(idx_fn: fn(&T) -> Vec<u8>, idx_namespace: &'a str) -> Self {
        UniqueIndex {
            index: idx_fn,
            idx_map: Map::new(idx_namespace),
        }
    }
}

impl<'a, T> Index<T> for UniqueIndex<'a, T>
where
    T: Serialize + DeserializeOwned + Clone,
{
    fn save(&self, store: &mut dyn Storage, pk: &[u8], data: &T) -> StdResult<()> {
        let idx = (self.index)(data);
        // error if this is already set
        self.idx_map
            .update(store, &idx, |existing| -> StdResult<_> {
                match existing {
                    Some(_) => Err(StdError::generic_err("Violates unique constraint on index")),
                    None => Ok(UniqueRef::<T> {
                        pk: pk.into(),
                        value: data.clone(),
                    }),
                }
            })?;
        Ok(())
    }

    fn remove(&self, store: &mut dyn Storage, _pk: &[u8], old_data: &T) -> StdResult<()> {
        let idx = (self.index)(old_data);
        self.idx_map.remove(store, &idx);
        Ok(())
    }
}

impl<'a, T> UniqueIndex<'a, T>
where
    T: Serialize + DeserializeOwned + Clone,
{
    /// returns all items that match this secondary index, always by pk Ascending
    pub fn item(&self, store: &dyn Storage, idx: &[u8]) -> StdResult<Option<KV<T>>> {
        let data = self
            .idx_map
            .may_load(store, &idx)?
            .map(|i| (i.pk.into(), i.value));
        Ok(data)
    }
}
