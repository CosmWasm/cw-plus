// this module requires iterator to be useful at all
#![cfg(feature = "iterator")]

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{from_slice, Binary, Order, Pair, StdError, StdResult, Storage};

use crate::helpers::namespaces_with_key;
use crate::keys::EmptyPrefix;
use crate::map::Map;
use crate::{Bound, Prefix, Prefixer, PrimaryKey, U32Key};

pub fn index_string(data: &str) -> Vec<u8> {
    data.as_bytes().to_vec()
}

pub fn index_tuple(name: &str, age: u32) -> (Vec<u8>, U32Key) {
    (index_string(name), U32Key::new(age))
}

pub fn index_triple(name: &str, age: u32, pk: Vec<u8>) -> (Vec<u8>, U32Key, Vec<u8>) {
    (index_string(name), U32Key::new(age), pk)
}

pub fn index_string_tuple(data1: &str, data2: &str) -> (Vec<u8>, Vec<u8>) {
    (index_string(data1), index_string(data2))
}

// Note: we cannot store traits with generic functions inside `Box<dyn Index>`,
// so I pull S: Storage to a top-level
pub trait Index<T>
where
    T: Serialize + DeserializeOwned + Clone,
{
    fn save(&self, store: &mut dyn Storage, pk: &[u8], data: &T) -> StdResult<()>;
    fn remove(&self, store: &mut dyn Storage, pk: &[u8], old_data: &T) -> StdResult<()>;
}

/// MultiIndex stores (namespace, index_name, idx_value, pk) -> b"pk_len".
/// Allows many values per index, and references pk.
/// The associated primary key value is stored in the main (pk_namespace) map,
/// which stores (namespace, pk_namespace, pk) -> value.
///
/// The stored pk_len is used to recover the pk from the index namespace, and perform
/// the secondary load of the associated value from the main map.
///
/// The MultiIndex definition must include a field for the pk. That is, the MultiIndex K value
/// is always a n-tuple (n >= 2) and its last element must be the pk.
/// The index function must therefore put the pk as last element, when generating the index.
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
    kv: Pair,
) -> StdResult<Pair<T>> {
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
    pub fn prefix(&self, p: K::Prefix) -> Prefix<T> {
        Prefix::with_deserialization_function(
            self.idx_namespace,
            &p.prefix(),
            self.pk_namespace,
            deserialize_multi_kv,
        )
    }

    pub fn sub_prefix(&self, p: K::SubPrefix) -> Prefix<T> {
        Prefix::with_deserialization_function(
            self.idx_namespace,
            &p.prefix(),
            self.pk_namespace,
            deserialize_multi_kv,
        )
    }

    pub fn index_key(&self, k: K) -> Vec<u8> {
        k.joined_key()
    }

    #[cfg(test)]
    pub fn count(&self, store: &dyn Storage, p: K::Prefix) -> usize {
        let prefix = self.prefix(p);
        prefix.keys(store, None, None, Order::Ascending).count()
    }

    #[cfg(test)]
    pub fn all_pks(&self, store: &dyn Storage, p: K::Prefix) -> Vec<Vec<u8>> {
        let prefix = self.prefix(p);
        prefix
            .keys(store, None, None, Order::Ascending)
            .collect::<Vec<Vec<u8>>>()
    }

    #[cfg(test)]
    pub fn all_items(&self, store: &dyn Storage, p: K::Prefix) -> StdResult<Vec<Pair<T>>> {
        let prefix = self.prefix(p);
        prefix.range(store, None, None, Order::Ascending).collect()
    }
}

// short-cut for simple keys, rather than .prefix(()).range(...)
impl<'a, K, T> MultiIndex<'a, K, T>
where
    T: Serialize + DeserializeOwned + Clone,
    K: PrimaryKey<'a>,
    K::SubPrefix: EmptyPrefix,
{
    // I would prefer not to copy code from Prefix, but no other way
    // with lifetimes (create Prefix inside function and return ref = no no)
    pub fn range<'c>(
        &'c self,
        store: &'c dyn Storage,
        min: Option<Bound>,
        max: Option<Bound>,
        order: Order,
    ) -> Box<dyn Iterator<Item = StdResult<Pair<T>>> + 'c>
    where
        T: 'c,
    {
        self.sub_prefix(K::SubPrefix::new())
            .range(store, min, max, order)
    }

    pub fn keys<'c>(
        &'c self,
        store: &'c dyn Storage,
        min: Option<Bound>,
        max: Option<Bound>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Vec<u8>> + 'c> {
        self.sub_prefix(K::SubPrefix::new())
            .keys(store, min, max, order)
    }
}

#[derive(Deserialize, Serialize)]
pub(crate) struct UniqueRef<T> {
    // note, we collapse the pk - combining everything under the namespace - even if it is composite
    pk: Binary,
    value: T,
}

/// UniqueIndex stores (namespace, index_name, idx_value) -> {key, value}
/// Allows one value per index (i.e. unique) and copies pk and data
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

fn deserialize_unique_kv<T: DeserializeOwned>(kv: Pair) -> StdResult<Pair<T>> {
    let (_, v) = kv;
    let t = from_slice::<UniqueRef<T>>(&v)?;
    Ok((t.pk.into(), t.value))
}

impl<'a, K, T> UniqueIndex<'a, K, T>
where
    T: Serialize + DeserializeOwned + Clone,
    K: PrimaryKey<'a>,
{
    pub fn index_key(&self, k: K) -> Vec<u8> {
        k.joined_key()
    }

    pub fn prefix(&self, p: K::Prefix) -> Prefix<T> {
        Prefix::with_deserialization_function(self.idx_namespace, &p.prefix(), &[], |_, _, kv| {
            deserialize_unique_kv(kv)
        })
    }

    pub fn sub_prefix(&self, p: K::SubPrefix) -> Prefix<T> {
        Prefix::with_deserialization_function(self.idx_namespace, &p.prefix(), &[], |_, _, kv| {
            deserialize_unique_kv(kv)
        })
    }

    /// returns all items that match this secondary index, always by pk Ascending
    pub fn item(&self, store: &dyn Storage, idx: K) -> StdResult<Option<Pair<T>>> {
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
    K::SubPrefix: EmptyPrefix,
{
    // I would prefer not to copy code from Prefix, but no other way
    // with lifetimes (create Prefix inside function and return ref = no no)
    pub fn range<'c>(
        &self,
        store: &'c dyn Storage,
        min: Option<Bound>,
        max: Option<Bound>,
        order: Order,
    ) -> Box<dyn Iterator<Item = StdResult<Pair<T>>> + 'c>
    where
        T: 'c,
    {
        self.sub_prefix(K::SubPrefix::new())
            .range(store, min, max, order)
    }

    pub fn keys<'c>(
        &self,
        store: &'c dyn Storage,
        min: Option<Bound>,
        max: Option<Bound>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Vec<u8>> + 'c> {
        self.sub_prefix(K::SubPrefix::new())
            .keys(store, min, max, order)
    }
}
