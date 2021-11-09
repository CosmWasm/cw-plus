// this module requires iterator to be useful at all
#![cfg(feature = "iterator")]

use std::marker::PhantomData;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{from_slice, Binary, Order, Record, StdError, StdResult, Storage};

use crate::de::KeyDeserialize;
use crate::helpers::namespaces_with_key;
use crate::iter_helpers::deserialize_kv;
use crate::map::Map;
use crate::prefix::{namespaced_prefix_range, PrefixBound};
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

fn deserialize_multi_kv<T: DeserializeOwned>(
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
    pub fn prefix(&self, p: K::Prefix) -> Prefix<Vec<u8>, T> {
        Prefix::with_deserialization_function(
            self.idx_namespace,
            &p.prefix(),
            self.pk_namespace,
            deserialize_multi_kv,
        )
    }

    pub fn sub_prefix(&self, p: K::SubPrefix) -> Prefix<Vec<u8>, T> {
        Prefix::with_deserialization_function(
            self.idx_namespace,
            &p.prefix(),
            self.pk_namespace,
            deserialize_multi_kv,
        )
    }

    fn no_prefix(&self) -> Prefix<Vec<u8>, T> {
        Prefix::with_deserialization_function(
            self.idx_namespace,
            &[],
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

    /// while range assumes you set the prefix to one element and call range over the last one,
    /// prefix_range accepts bounds for the lowest and highest elements of the Prefix we wish to
    /// accept, and iterates over those. There are some issues that distinguish these to and blindly
    /// casting to Vec<u8> doesn't solve them.
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
            .map(move |kv| (deserialize_multi_kv)(store, self.pk_namespace, kv));
        Box::new(mapped)
    }
}

#[cfg(feature = "iterator")]
impl<'a, K, T> MultiIndex<'a, K, T>
where
    T: Serialize + DeserializeOwned + Clone,
    K: PrimaryKey<'a> + KeyDeserialize,
{
    /// while range_de assumes you set the prefix to one element and call range over the last one,
    /// prefix_range_de accepts bounds for the lowest and highest elements of the Prefix we wish to
    /// accept, and iterates over those. There are some issues that distinguish these to and blindly
    /// casting to Vec<u8> doesn't solve them.
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
        Prefix::new(self.idx_namespace, &[])
    }
}

/// UniqueRef stores Binary(Vec[u8]) representation of private key and index value
#[derive(Deserialize, Serialize)]
pub(crate) struct UniqueRef<T> {
    // note, we collapse the pk - combining everything under the namespace - even if it is composite
    pk: Binary,
    value: T,
}

/// UniqueIndex stores (namespace, index_name, idx_value) -> {key, value}
/// Allows one value per index (i.e. unique) and copies pk and data
/// The optional PK type defines the type of Primary Key deserialization.
pub struct UniqueIndex<'a, K, T, PK = ()> {
    index: fn(&T) -> K,
    idx_map: Map<'a, K, UniqueRef<T>>,
    idx_namespace: &'a [u8],
    _phantom: PhantomData<PK>,
}

impl<'a, K, T, PK> UniqueIndex<'a, K, T, PK> {
    // TODO: make this a const fn
    /// Create a new UniqueIndex
    ///
    /// idx_fn - lambda creating index key from index value
    /// idx_namespace - prefix for the index value
    ///
    /// ## Example:
    ///
    /// ```rust
    /// use cw_storage_plus::{U32Key, UniqueIndex};
    ///
    /// struct Data {
    ///     pub name: String,
    ///     pub age: u32,
    /// }
    ///
    /// UniqueIndex::<_, _, ()>::new(|d: &Data| U32Key::new(d.age), "data__age");
    /// ```
    pub fn new(idx_fn: fn(&T) -> K, idx_namespace: &'a str) -> Self {
        UniqueIndex {
            index: idx_fn,
            idx_map: Map::new(idx_namespace),
            idx_namespace: idx_namespace.as_bytes(),
            _phantom: PhantomData,
        }
    }
}

impl<'a, K, T, PK> Index<T> for UniqueIndex<'a, K, T, PK>
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

fn deserialize_unique_v<T: DeserializeOwned>(kv: Record) -> StdResult<Record<T>> {
    let (_, v) = kv;
    let t = from_slice::<UniqueRef<T>>(&v)?;
    Ok((t.pk.to_vec(), t.value))
}

fn deserialize_unique_kv<T: DeserializeOwned, K: KeyDeserialize>(
    kv: Record,
) -> StdResult<(K::Output, T)> {
    let (_, v) = kv;
    let t = from_slice::<UniqueRef<T>>(&v)?;
    Ok((K::from_vec(t.pk.to_vec())?, t.value))
}

impl<'a, K, T, PK> UniqueIndex<'a, K, T, PK>
where
    T: Serialize + DeserializeOwned + Clone,
    K: PrimaryKey<'a>,
{
    pub fn index_key(&self, k: K) -> Vec<u8> {
        k.joined_key()
    }

    pub fn prefix(&self, p: K::Prefix) -> Prefix<Vec<u8>, T> {
        Prefix::with_deserialization_function(self.idx_namespace, &p.prefix(), &[], |_, _, kv| {
            deserialize_unique_v(kv)
        })
    }

    pub fn sub_prefix(&self, p: K::SubPrefix) -> Prefix<Vec<u8>, T> {
        Prefix::with_deserialization_function(self.idx_namespace, &p.prefix(), &[], |_, _, kv| {
            deserialize_unique_v(kv)
        })
    }

    fn no_prefix(&self) -> Prefix<Vec<u8>, T> {
        Prefix::with_deserialization_function(self.idx_namespace, &[], &[], |_, _, kv| {
            deserialize_unique_v(kv)
        })
    }

    /// returns all items that match this secondary index, always by pk Ascending
    pub fn item(&self, store: &dyn Storage, idx: K) -> StdResult<Option<Record<T>>> {
        let data = self
            .idx_map
            .may_load(store, idx)?
            .map(|i| (i.pk.into(), i.value));
        Ok(data)
    }
}

// short-cut for simple keys, rather than .prefix(()).range(...)
impl<'a, K, T, PK> UniqueIndex<'a, K, T, PK>
where
    T: Serialize + DeserializeOwned + Clone,
    K: PrimaryKey<'a>,
{
    // I would prefer not to copy code from Prefix, but no other way
    // with lifetimes (create Prefix inside function and return ref = no no)
    pub fn range<'c>(
        &self,
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
        &self,
        store: &'c dyn Storage,
        min: Option<Bound>,
        max: Option<Bound>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Vec<u8>> + 'c> {
        self.no_prefix().keys(store, min, max, order)
    }
}

#[cfg(feature = "iterator")]
impl<'a, K, T, PK> UniqueIndex<'a, K, T, PK>
where
    PK: PrimaryKey<'a> + KeyDeserialize,
    T: Serialize + DeserializeOwned + Clone,
    K: PrimaryKey<'a>,
{
    /// while range_de assumes you set the prefix to one element and call range over the last one,
    /// prefix_range_de accepts bounds for the lowest and highest elements of the Prefix we wish to
    /// accept, and iterates over those. There are some issues that distinguish these to and blindly
    /// casting to Vec<u8> doesn't solve them.
    pub fn prefix_range_de<'c>(
        &self,
        store: &'c dyn Storage,
        min: Option<PrefixBound<'a, PK::Prefix>>,
        max: Option<PrefixBound<'a, PK::Prefix>>,
        order: cosmwasm_std::Order,
    ) -> Box<dyn Iterator<Item = StdResult<(PK::Output, T)>> + 'c>
    where
        T: 'c,
        'a: 'c,
        K: 'c,
        PK: 'c,
        PK::Output: 'static,
    {
        let mapped = namespaced_prefix_range(store, self.idx_namespace, min, max, order)
            .map(deserialize_kv::<PK, T>);
        Box::new(mapped)
    }

    pub fn range_de<'c>(
        &self,
        store: &'c dyn Storage,
        min: Option<Bound>,
        max: Option<Bound>,
        order: cosmwasm_std::Order,
    ) -> Box<dyn Iterator<Item = StdResult<(PK::Output, T)>> + 'c>
    where
        T: 'c,
        PK::Output: 'static,
    {
        self.no_prefix_de().range_de(store, min, max, order)
    }

    pub fn keys_de<'c>(
        &self,
        store: &'c dyn Storage,
        min: Option<Bound>,
        max: Option<Bound>,
        order: cosmwasm_std::Order,
    ) -> Box<dyn Iterator<Item = StdResult<PK::Output>> + 'c>
    where
        T: 'c,
        PK::Output: 'static,
    {
        self.no_prefix_de().keys_de(store, min, max, order)
    }

    pub fn prefix_de(&self, p: K::Prefix) -> Prefix<PK, T> {
        Prefix::with_deserialization_function(self.idx_namespace, &p.prefix(), &[], |_, _, kv| {
            deserialize_unique_kv::<_, PK>(kv)
        })
    }

    pub fn sub_prefix_de(&self, p: K::SubPrefix) -> Prefix<PK, T> {
        Prefix::with_deserialization_function(self.idx_namespace, &p.prefix(), &[], |_, _, kv| {
            deserialize_unique_kv::<_, PK>(kv)
        })
    }

    fn no_prefix_de(&self) -> Prefix<PK, T> {
        Prefix::with_deserialization_function(self.idx_namespace, &[], &[], |_, _, kv| {
            deserialize_unique_kv::<_, PK>(kv)
        })
    }
}
