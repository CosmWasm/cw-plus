// this module requires iterator to be useful at all
#![cfg(feature = "iterator")]

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{from_slice, Binary, Order, StdError, StdResult, Storage, KV};

use crate::helpers::namespaces_with_key;
use crate::keys::EmptyPrefix;
use crate::map::Map;
use crate::prefix::range_with_prefix;
use crate::{Bound, PkOwned, Prefix, Prefixer, PrimaryKey, U32Key};

pub fn index_string(data: &str) -> PkOwned {
    PkOwned(data.as_bytes().to_vec())
}

pub fn index_tuple(name: &str, age: u32) -> (PkOwned, U32Key) {
    (index_string(name), U32Key::new(age))
}

pub fn index_triple(name: &str, age: u32, pk: Vec<u8>) -> (PkOwned, U32Key, PkOwned) {
    (index_string(name), U32Key::new(age), PkOwned(pk))
}

pub fn index_string_tuple(data1: &str, data2: &str) -> (PkOwned, PkOwned) {
    (index_string(data1), index_string(data2))
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
    index: fn(&T, Vec<u8>) -> K,
    idx_namespace: &'a [u8],
    idx_map: Map<'a, K, u32>,
    pk_namespace: &'a [u8],
}

impl<'a, K, T> MultiIndex<'a, K, T>
where
    T: Serialize + DeserializeOwned + Clone,
{
    // TODO: make this a const fn
    pub fn new(
        idx_fn: fn(&T, Vec<u8>) -> K,
        pk_namespace: &'a str,
        idx_namespace: &'a str,
    ) -> Self {
        MultiIndex {
            index: idx_fn,
            idx_namespace: idx_namespace.as_bytes(),
            idx_map: Map::new(idx_namespace),
            pk_namespace: pk_namespace.as_bytes(),
        }
    }
}

fn deserialize_multi_kv<T: DeserializeOwned>(
    store: &dyn Storage,
    pk_namespace: &[u8],
    kv: KV,
) -> StdResult<KV<T>> {
    let (key, pk_len) = kv;

    // Deserialize pk_len
    let pk_len = from_slice::<u32>(pk_len.as_slice())?;

    // Recover pk from last part of k
    let offset = key.len() - pk_len as usize;
    let pk = &key[offset..];

    let full_key = namespaces_with_key(&[pk_namespace], pk);

    let v = store
        .get(&full_key)
        .ok_or_else(|| StdError::generic_err("pk not found"))?;
    let v = from_slice::<T>(&v)?;

    Ok((pk.into(), v))
}

impl<'a, K, T> Index<T> for MultiIndex<'a, K, T>
where
    T: Serialize + DeserializeOwned + Clone,
    K: PrimaryKey<'a>,
{
    fn save(&self, store: &mut dyn Storage, pk: &[u8], data: &T) -> StdResult<()> {
        let idx = (self.index)(data, pk.to_vec());
        self.idx_map.save(store, idx, &(pk.len() as u32))
    }

    fn remove(&self, store: &mut dyn Storage, pk: &[u8], old_data: &T) -> StdResult<()> {
        let idx = (self.index)(old_data, pk.to_vec());
        self.idx_map.remove(store, idx);
        Ok(())
    }
}

impl<'a, K, T> MultiIndex<'a, K, T>
where
    T: Serialize + DeserializeOwned + Clone,
    K: PrimaryKey<'a>,
{
    // FIXME?: Use range() syntax / return values
    // FIXME?: Move to Prefix<T> for ergonomics
    // FIXME: Recover pk from (last part of) k
    pub fn pks<'c>(
        &self,
        store: &'c dyn Storage,
        prefix: Prefix<T>,
        min: Option<Bound>,
        max: Option<Bound>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Vec<u8>> + 'c> {
        let mapped = range_with_prefix(store, &prefix, min, max, order).map(|(k, _)| k);
        Box::new(mapped)
    }

    /// returns all items that match this secondary index, always by pk Ascending
    // pub fn items<'c>(
    //     &'c self,
    //     store: &'c dyn Storage,
    //     idx: &[u8],
    //     min: Option<Bound>,
    //     max: Option<Bound>,
    //     order: Order,
    // ) -> Box<dyn Iterator<Item = StdResult<KV<T>>> + 'c> {
    //     let mapped = self.pks(store, idx, min, max, order).map(move |pk| {
    //         let v = self.pk_map.load(store, &pk)?;
    //         Ok((pk, v))
    //     });
    //     Box::new(mapped)
    // }

    pub fn prefix(&self, p: K::Prefix) -> Prefix<T> {
        Prefix::new_de_fn(
            self.idx_namespace,
            &p.prefix(),
            self.pk_namespace,
            deserialize_multi_kv,
        )
    }

    // #[cfg(test)]
    // pub fn count<'c>(&self, store: &'c dyn Storage, idx: &[u8]) -> usize {
    //     self.pks(store, idx, None, None, Order::Ascending).count()
    // }

    // #[cfg(test)]
    // pub fn all_pks<'c>(&self, store: &'c dyn Storage, idx: &[u8]) -> Vec<Vec<u8>> {
    //     self.pks(store, idx, None, None, Order::Ascending).collect()
    // }

    // #[cfg(test)]
    // pub fn all_items<'c>(&self, store: &'c dyn Storage, idx: &[u8]) -> StdResult<Vec<KV<T>>> {
    //     self.items(store, idx, None, None, Order::Ascending)
    //         .collect()
    // }
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
        &'c self,
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

pub struct UniqueIndex<'a, K, T> {
    index: fn(&T) -> K,
    idx_map: Map<'a, K, UniqueRef<T>>,
    idx_namespace: &'a [u8],
}

impl<'a, K, T> UniqueIndex<'a, K, T> {
    // TODO: make this a const fn
    pub fn new(idx_fn: fn(&T) -> K, idx_namespace: &'a str) -> Self {
        UniqueIndex {
            index: idx_fn,
            idx_map: Map::new(idx_namespace),
            idx_namespace: idx_namespace.as_bytes(),
        }
    }
}

impl<'a, K, T> Index<T> for UniqueIndex<'a, K, T>
where
    T: Serialize + DeserializeOwned + Clone,
    K: PrimaryKey<'a>,
{
    fn save(&self, store: &mut dyn Storage, pk: &[u8], data: &T) -> StdResult<()> {
        let idx = (self.index)(data);
        // error if this is already set
        self.idx_map
            .update(store, idx, |existing| -> StdResult<_> {
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
        self.idx_map.remove(store, idx);
        Ok(())
    }
}

fn deserialize_unique_kv<T: DeserializeOwned>(kv: KV) -> StdResult<KV<T>> {
    let (_, v) = kv;
    let t = from_slice::<UniqueRef<T>>(&v)?;
    Ok((t.pk.into(), t.value))
}

impl<'a, K, T> UniqueIndex<'a, K, T>
where
    T: Serialize + DeserializeOwned + Clone,
    K: PrimaryKey<'a>,
{
    pub fn prefix(&self, p: K::Prefix) -> Prefix<T> {
        Prefix::new_de_fn(self.idx_namespace, &p.prefix(), &[], |_, _, kv| {
            deserialize_unique_kv(kv)
        })
    }

    /// returns all items that match this secondary index, always by pk Ascending
    pub fn item(&self, store: &dyn Storage, idx: K) -> StdResult<Option<KV<T>>> {
        let data = self
            .idx_map
            .may_load(store, idx)?
            .map(|i| (i.pk.into(), i.value));
        Ok(data)
    }
}

// short-cut for simple keys, rather than .prefix(()).range(...)
impl<'a, K, T> UniqueIndex<'a, K, T>
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
    ) -> Box<dyn Iterator<Item = StdResult<KV<T>>> + 'c>
    where
        T: 'c,
    {
        self.prefix(K::Prefix::new()).range(store, min, max, order)
    }
}
