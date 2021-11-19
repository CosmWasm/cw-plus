// this module requires iterator to be useful at all
#![cfg(feature = "iterator")]

use std::marker::PhantomData;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{from_slice, Binary, Order, Record, StdError, StdResult, Storage};

use crate::de::KeyDeserialize;
use crate::iter_helpers::deserialize_kv;
use crate::map::Map;
use crate::prefix::{namespaced_prefix_range, PrefixBound};
use crate::{Bound, Index, Prefix, Prefixer, PrimaryKey};

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
    // FIXME: Return `k` here instead of `t.pk` (be consistent with `Map` behaviour)
    Ok((t.pk.to_vec(), t.value))
}

fn deserialize_unique_kv<K: KeyDeserialize, T: DeserializeOwned>(
    kv: Record,
) -> StdResult<(K::Output, T)> {
    let (_, v) = kv;
    let t = from_slice::<UniqueRef<T>>(&v)?;
    // FIXME: Return `k` deserialization here instead of `t.pk` (be consistent with `deserialize_multi_kv` and `Map` behaviour)
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
            deserialize_unique_kv::<PK, _>(kv)
        })
    }

    pub fn sub_prefix_de(&self, p: K::SubPrefix) -> Prefix<PK, T> {
        Prefix::with_deserialization_function(self.idx_namespace, &p.prefix(), &[], |_, _, kv| {
            deserialize_unique_kv::<PK, _>(kv)
        })
    }

    fn no_prefix_de(&self) -> Prefix<PK, T> {
        Prefix::with_deserialization_function(self.idx_namespace, &[], &[], |_, _, kv| {
            deserialize_unique_kv::<PK, _>(kv)
        })
    }
}
