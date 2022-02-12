// this module requires iterator to be useful at all
#![cfg(feature = "iterator")]

use serde::de::DeserializeOwned;
use serde::Serialize;

use cosmwasm_std::{from_slice, Order, Record, StdError, StdResult, Storage};

use crate::bound::PrefixBound;
use crate::de::KeyDeserialize;
use crate::helpers::namespaces_with_key;
use crate::iter_helpers::deserialize_kv;
use crate::map::Map;
use crate::prefix::namespaced_prefix_range;
use crate::{Bound, Index, Prefix, Prefixer, PrimaryKey};
use std::marker::PhantomData;

/// MultiIndex stores (namespace, index_name, idx_value, pk) -> b"pk_len".
/// Allows many values per index, and references pk.
/// The associated primary key value is stored in the main (pk_namespace) map,
/// which stores (namespace, pk_namespace, pk) -> value.
///
/// The stored pk_len is used to recover the pk from the index namespace, and perform
/// the secondary load of the associated value from the main map.
///
/// The (optional) PK type defines the type of Primary Key deserialization.
pub struct MultiIndex<'a, IK, T, PK = ()> {
    index: fn(&T) -> IK,
    idx_namespace: &'a [u8],
    // note, we collapse the ik - combining everything under the namespace - and concatenating the pk
    idx_map: Map<'a, Vec<u8>, u32>,
    pk_namespace: &'a [u8],
    phantom: PhantomData<PK>,
}

impl<'a, IK, T, PK> MultiIndex<'a, IK, T, PK>
where
    T: Serialize + DeserializeOwned + Clone,
{
    // TODO: make this a const fn
    /// Create a new MultiIndex
    ///
    /// idx_fn - lambda creating index key from value
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
    /// let index: MultiIndex<_, _, String> = MultiIndex::new(
    ///     |d: &Data| d.age,
    ///     "age",
    ///     "age__owner",
    /// );
    /// ```
    pub fn new(idx_fn: fn(&T) -> IK, pk_namespace: &'a str, idx_namespace: &'a str) -> Self {
        MultiIndex {
            index: idx_fn,
            idx_namespace: idx_namespace.as_bytes(),
            idx_map: Map::new(idx_namespace),
            pk_namespace: pk_namespace.as_bytes(),
            phantom: PhantomData,
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

    // We return deserialized `pk` here for consistency
    Ok((K::from_slice(pk)?, v))
}

impl<'a, IK, T, PK> Index<T> for MultiIndex<'a, IK, T, PK>
where
    T: Serialize + DeserializeOwned + Clone,
    IK: PrimaryKey<'a>,
{
    fn save(&self, store: &mut dyn Storage, pk: &[u8], data: &T) -> StdResult<()> {
        let idx = (self.index)(data).joined_extra_key(pk);
        self.idx_map.save(store, idx, &(pk.len() as u32))
    }

    fn remove(&self, store: &mut dyn Storage, pk: &[u8], old_data: &T) -> StdResult<()> {
        let idx = (self.index)(old_data).joined_extra_key(pk);
        self.idx_map.remove(store, idx);
        Ok(())
    }
}

impl<'a, IK, T, PK> MultiIndex<'a, IK, T, PK>
where
    T: Serialize + DeserializeOwned + Clone,
    IK: PrimaryKey<'a> + Prefixer<'a>,
{
    fn no_prefix_raw(&self) -> Prefix<Vec<u8>, T, (IK, PK)> {
        Prefix::with_deserialization_functions(
            self.idx_namespace,
            &[],
            self.pk_namespace,
            deserialize_multi_v,
            deserialize_multi_v,
        )
    }
}

impl<'a, IK, T, PK> MultiIndex<'a, IK, T, PK>
where
    PK: PrimaryKey<'a> + KeyDeserialize,
    T: Serialize + DeserializeOwned + Clone,
    IK: PrimaryKey<'a> + Prefixer<'a>,
{
    pub fn index_key(&self, k: IK) -> Vec<u8> {
        k.joined_extra_key(b"")
    }

    #[cfg(test)]
    pub fn count(&self, store: &dyn Storage, p: IK) -> usize {
        let prefix = self.prefix(p);
        prefix.keys_raw(store, None, None, Order::Ascending).count()
    }

    #[cfg(test)]
    pub fn all_pks(&self, store: &dyn Storage, p: IK) -> Vec<Vec<u8>> {
        let prefix = self.prefix(p);
        prefix
            .keys_raw(store, None, None, Order::Ascending)
            .collect::<Vec<Vec<u8>>>()
    }

    #[cfg(test)]
    pub fn all_items(&self, store: &dyn Storage, p: IK) -> StdResult<Vec<Record<T>>> {
        let prefix = self.prefix(p);
        prefix
            .range_raw(store, None, None, Order::Ascending)
            .collect()
    }
}

// short-cut for simple keys, rather than .prefix(()).range_raw(...)
impl<'a, IK, T, PK> MultiIndex<'a, IK, T, PK>
where
    T: Serialize + DeserializeOwned + Clone,
    IK: PrimaryKey<'a> + Prefixer<'a> + KeyDeserialize,
    PK: PrimaryKey<'a> + KeyDeserialize,
{
    // I would prefer not to copy code from Prefix, but no other way
    // with lifetimes (create Prefix inside function and return ref = no no)
    pub fn range_raw<'c>(
        &'c self,
        store: &'c dyn Storage,
        min: Option<Bound<'a, (IK, PK)>>,
        max: Option<Bound<'a, (IK, PK)>>,
        order: Order,
    ) -> Box<dyn Iterator<Item = StdResult<Record<T>>> + 'c>
    where
        T: 'c,
    {
        self.no_prefix_raw().range_raw(store, min, max, order)
    }

    pub fn keys_raw<'c>(
        &'c self,
        store: &'c dyn Storage,
        min: Option<Bound<'a, (IK, PK)>>,
        max: Option<Bound<'a, (IK, PK)>>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Vec<u8>> + 'c> {
        self.no_prefix_raw().keys_raw(store, min, max, order)
    }

    /// While `range_raw` over a `prefix` fixes the prefix to one element and iterates over the
    /// remaining, `prefix_range_raw` accepts bounds for the lowest and highest elements of the
    /// `Prefix` itself, and iterates over those (inclusively or exclusively, depending on
    /// `PrefixBound`).
    /// There are some issues that distinguish these two, and blindly casting to `Vec<u8>` doesn't
    /// solve them.
    pub fn prefix_range_raw<'c>(
        &'c self,
        store: &'c dyn Storage,
        min: Option<PrefixBound<'a, IK>>,
        max: Option<PrefixBound<'a, IK>>,
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
impl<'a, IK, T, PK> MultiIndex<'a, IK, T, PK>
where
    PK: PrimaryKey<'a> + KeyDeserialize,
    T: Serialize + DeserializeOwned + Clone,
    IK: PrimaryKey<'a> + Prefixer<'a>,
{
    pub fn prefix(&self, p: IK) -> Prefix<PK, T, PK> {
        Prefix::with_deserialization_functions(
            self.idx_namespace,
            &p.prefix(),
            self.pk_namespace,
            deserialize_multi_kv::<PK, T>,
            deserialize_multi_v,
        )
    }

    pub fn sub_prefix(&self, p: IK::Prefix) -> Prefix<PK, T, (IK::Suffix, PK)> {
        Prefix::with_deserialization_functions(
            self.idx_namespace,
            &p.prefix(),
            self.pk_namespace,
            deserialize_multi_kv::<PK, T>,
            deserialize_multi_v,
        )
    }
}

#[cfg(feature = "iterator")]
impl<'a, IK, T, PK> MultiIndex<'a, IK, T, PK>
where
    PK: PrimaryKey<'a> + KeyDeserialize,
    T: Serialize + DeserializeOwned + Clone,
    IK: PrimaryKey<'a> + KeyDeserialize + Prefixer<'a>,
{
    /// While `range` over a `prefix` fixes the prefix to one element and iterates over the
    /// remaining, `prefix_range` accepts bounds for the lowest and highest elements of the
    /// `Prefix` itself, and iterates over those (inclusively or exclusively, depending on
    /// `PrefixBound`).
    /// There are some issues that distinguish these two, and blindly casting to `Vec<u8>` doesn't
    /// solve them.
    pub fn prefix_range<'c>(
        &self,
        store: &'c dyn Storage,
        min: Option<PrefixBound<'a, IK>>,
        max: Option<PrefixBound<'a, IK>>,
        order: cosmwasm_std::Order,
    ) -> Box<dyn Iterator<Item = StdResult<(PK::Output, T)>> + 'c>
    where
        T: 'c,
        'a: 'c,
        IK: 'c,
        PK: 'c,
        PK::Output: 'static,
    {
        let mapped = namespaced_prefix_range(store, self.idx_namespace, min, max, order)
            .map(deserialize_kv::<PK, T>);
        Box::new(mapped)
    }

    pub fn range<'c>(
        &self,
        store: &'c dyn Storage,
        min: Option<Bound<'a, (IK, PK)>>,
        max: Option<Bound<'a, (IK, PK)>>,
        order: cosmwasm_std::Order,
    ) -> Box<dyn Iterator<Item = StdResult<(PK::Output, T)>> + 'c>
    where
        T: 'c,
        PK::Output: 'static,
    {
        self.no_prefix().range(store, min, max, order)
    }

    pub fn keys<'c>(
        &self,
        store: &'c dyn Storage,
        min: Option<Bound<'a, (IK, PK)>>,
        max: Option<Bound<'a, (IK, PK)>>,
        order: cosmwasm_std::Order,
    ) -> Box<dyn Iterator<Item = StdResult<PK::Output>> + 'c>
    where
        T: 'c,
        PK::Output: 'static,
    {
        self.no_prefix().keys(store, min, max, order)
    }

    fn no_prefix(&self) -> Prefix<PK, T, (IK, PK)> {
        Prefix::with_deserialization_functions(
            self.idx_namespace,
            &[],
            self.pk_namespace,
            deserialize_multi_kv::<PK, T>,
            deserialize_multi_v,
        )
    }
}
