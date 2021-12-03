// this module requires iterator to be useful at all
#![cfg(feature = "iterator")]

use serde::de::DeserializeOwned;
use serde::Serialize;

use cosmwasm_std::{from_slice, Order, Record, StdError, StdResult, Storage};

use crate::de::KeyDeserialize;
use crate::helpers::namespaces_with_key;
use crate::iter_helpers::deserialize_kv;
use crate::map::Map;
use crate::prefix::{namespaced_prefix_range, PrefixBound};
use crate::{Bound, Index, Prefix, Prefixer, PrimaryKey};

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
    /// Create a new MultiIndex
    ///
    /// idx_fn - lambda creating index key from value (first argument) and primary key (second argument)
    /// pk_namespace - prefix for the primary key
    /// idx_namespace - prefix for the index value
    ///
    /// ## Example:
    ///
    /// ```rust
    /// use cw_storage_plus::MultiIndex;
    /// use serde::{Deserialize, Serialize};
    ///
    /// #[derive(Deserialize, Serialize, Clone)]
    /// struct Data {
    ///     pub name: String,
    ///     pub age: u32,
    /// }
    ///
    /// MultiIndex::new(
    ///     |d: &Data, k: Vec<u8>| (d.age, k),
    ///     "age",
    ///     "age__owner",
    /// );
    /// ```
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

fn deserialize_multi_v<T: DeserializeOwned>(
    store: &dyn Storage,
    pk_namespace: &[u8],
    kv: Record,
) -> StdResult<Record<T>> {
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

    Ok((pk.to_vec(), v))
}

fn deserialize_multi_kv<K: KeyDeserialize, T: DeserializeOwned>(
    store: &dyn Storage,
    pk_namespace: &[u8],
    kv: Record,
) -> StdResult<(K::Output, T)> {
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

    // FIXME: Return `pk` here instead of `key` for consistency
    Ok((K::from_vec(key)?, v))
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
    pub fn prefix(&self, p: K::Prefix) -> Prefix<Vec<u8>, T> {
        Prefix::with_deserialization_function(
            self.idx_namespace,
            &p.prefix(),
            self.pk_namespace,
            deserialize_multi_v,
        )
    }

    pub fn sub_prefix(&self, p: K::SubPrefix) -> Prefix<Vec<u8>, T> {
        Prefix::with_deserialization_function(
            self.idx_namespace,
            &p.prefix(),
            self.pk_namespace,
            deserialize_multi_v,
        )
    }

    fn no_prefix(&self) -> Prefix<Vec<u8>, T> {
        Prefix::with_deserialization_function(
            self.idx_namespace,
            &[],
            self.pk_namespace,
            deserialize_multi_v,
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
    pub fn all_items(&self, store: &dyn Storage, p: K::Prefix) -> StdResult<Vec<Record<T>>> {
        let prefix = self.prefix(p);
        prefix.range(store, None, None, Order::Ascending).collect()
    }
}

// short-cut for simple keys, rather than .prefix(()).range(...)
impl<'a, K, T> MultiIndex<'a, K, T>
where
    T: Serialize + DeserializeOwned + Clone,
    K: PrimaryKey<'a>,
{
    // I would prefer not to copy code from Prefix, but no other way
    // with lifetimes (create Prefix inside function and return ref = no no)
    pub fn range<'c>(
        &'c self,
        store: &'c dyn Storage,
        min: Option<Bound>,
        max: Option<Bound>,
        order: Order,
    ) -> Box<dyn Iterator<Item = StdResult<Record<T>>> + 'c>
    where
        T: 'c,
    {
        self.no_prefix().range(store, min, max, order)
    }

    pub fn keys<'c>(
        &'c self,
        store: &'c dyn Storage,
        min: Option<Bound>,
        max: Option<Bound>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Vec<u8>> + 'c> {
        self.no_prefix().keys(store, min, max, order)
    }

    /// While `range_de` over a `prefix_de` fixes the prefix to one element and iterates over the
    /// remaining, `prefix_range_de` accepts bounds for the lowest and highest elements of the
    /// `Prefix` itself, and iterates over those (inclusively or exclusively, depending on
    /// `PrefixBound`).
    /// There are some issues that distinguish these two, and blindly casting to `Vec<u8>` doesn't
    /// solve them.
    pub fn prefix_range<'c>(
        &'c self,
        store: &'c dyn Storage,
        min: Option<PrefixBound<'a, K::Prefix>>,
        max: Option<PrefixBound<'a, K::Prefix>>,
        order: cosmwasm_std::Order,
    ) -> Box<dyn Iterator<Item = StdResult<cosmwasm_std::Record<T>>> + 'c>
    where
        T: 'c,
        'a: 'c,
    {
        let mapped = namespaced_prefix_range(store, self.idx_namespace, min, max, order)
            .map(move |kv| (deserialize_multi_v)(store, self.pk_namespace, kv));
        Box::new(mapped)
    }
}

#[cfg(feature = "iterator")]
impl<'a, K, T> MultiIndex<'a, K, T>
where
    T: Serialize + DeserializeOwned + Clone,
    K: PrimaryKey<'a>,
{
    pub fn prefix_de(&self, p: K::Prefix) -> Prefix<K::Suffix, T> {
        Prefix::with_deserialization_function(
            self.idx_namespace,
            &p.prefix(),
            self.pk_namespace,
            deserialize_multi_kv::<K::Suffix, T>,
        )
    }

    pub fn sub_prefix_de(&self, p: K::SubPrefix) -> Prefix<K::SuperSuffix, T> {
        Prefix::with_deserialization_function(
            self.idx_namespace,
            &p.prefix(),
            self.pk_namespace,
            deserialize_multi_kv::<K::SuperSuffix, T>,
        )
    }
}

#[cfg(feature = "iterator")]
impl<'a, K, T> MultiIndex<'a, K, T>
where
    T: Serialize + DeserializeOwned + Clone,
    K: PrimaryKey<'a> + KeyDeserialize,
{
    /// While `range_de` over a `prefix_de` fixes the prefix to one element and iterates over the
    /// remaining, `prefix_range_de` accepts bounds for the lowest and highest elements of the
    /// `Prefix` itself, and iterates over those (inclusively or exclusively, depending on
    /// `PrefixBound`).
    /// There are some issues that distinguish these two, and blindly casting to `Vec<u8>` doesn't
    /// solve them.
    pub fn prefix_range_de<'c>(
        &self,
        store: &'c dyn Storage,
        min: Option<PrefixBound<'a, K::Prefix>>,
        max: Option<PrefixBound<'a, K::Prefix>>,
        order: cosmwasm_std::Order,
    ) -> Box<dyn Iterator<Item = StdResult<(K::Output, T)>> + 'c>
    where
        T: 'c,
        'a: 'c,
        K: 'c,
        K::Output: 'static,
    {
        let mapped = namespaced_prefix_range(store, self.idx_namespace, min, max, order)
            .map(deserialize_kv::<K, T>);
        Box::new(mapped)
    }

    pub fn range_de<'c>(
        &self,
        store: &'c dyn Storage,
        min: Option<Bound>,
        max: Option<Bound>,
        order: cosmwasm_std::Order,
    ) -> Box<dyn Iterator<Item = StdResult<(K::Output, T)>> + 'c>
    where
        T: 'c,
        K::Output: 'static,
    {
        self.no_prefix_de().range_de(store, min, max, order)
    }

    pub fn keys_de<'c>(
        &self,
        store: &'c dyn Storage,
        min: Option<Bound>,
        max: Option<Bound>,
        order: cosmwasm_std::Order,
    ) -> Box<dyn Iterator<Item = StdResult<K::Output>> + 'c>
    where
        T: 'c,
        K::Output: 'static,
    {
        self.no_prefix_de().keys_de(store, min, max, order)
    }

    fn no_prefix_de(&self) -> Prefix<K, T> {
        Prefix::with_deserialization_function(
            self.idx_namespace,
            &[],
            self.pk_namespace,
            deserialize_multi_kv::<K, T>,
        )
    }
}
