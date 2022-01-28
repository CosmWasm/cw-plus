// this module requires iterator to be useful at all
#![cfg(feature = "iterator")]

use std::marker::PhantomData;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{from_slice, Binary, Order, Record, StdError, StdResult, Storage};

use crate::bound::PrefixBound;
use crate::de::KeyDeserialize;
use crate::iter_helpers::deserialize_kv;
use crate::map::Map;
use crate::prefix::namespaced_prefix_range;
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
pub struct UniqueIndex<'a, IK, T, PK = ()> {
    index: fn(&T) -> IK,
    idx_map: Map<'a, IK, UniqueRef<T>>,
    idx_namespace: &'a [u8],
    phantom: PhantomData<PK>,
}

impl<'a, IK, T, PK> UniqueIndex<'a, IK, T, PK> {
    // TODO: make this a const fn
    /// Create a new UniqueIndex
    ///
    /// idx_fn - lambda creating index key from index value
    /// idx_namespace - prefix for the index value
    ///
    /// ## Example:
    ///
    /// ```rust
    /// use cw_storage_plus::UniqueIndex;
    ///
    /// struct Data {
    ///     pub name: String,
    ///     pub age: u32,
    /// }
    ///
    /// UniqueIndex::<_, _, ()>::new(|d: &Data| d.age, "data__age");
    /// ```
    pub fn new(idx_fn: fn(&T) -> IK, idx_namespace: &'a str) -> Self {
        UniqueIndex {
            index: idx_fn,
            idx_map: Map::new(idx_namespace),
            idx_namespace: idx_namespace.as_bytes(),
            phantom: PhantomData,
        }
    }
}

impl<'a, IK, T, PK> Index<T> for UniqueIndex<'a, IK, T, PK>
where
    T: Serialize + DeserializeOwned + Clone,
    IK: PrimaryKey<'a>,
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
    Ok((t.pk.0, t.value))
}

fn deserialize_unique_kv<K: KeyDeserialize, T: DeserializeOwned>(
    kv: Record,
) -> StdResult<(K::Output, T)> {
    let (_, v) = kv;
    let t = from_slice::<UniqueRef<T>>(&v)?;
    Ok((K::from_vec(t.pk.0)?, t.value))
}

impl<'a, IK, T, PK> UniqueIndex<'a, IK, T, PK>
where
    T: Serialize + DeserializeOwned + Clone,
    IK: PrimaryKey<'a>,
{
    pub fn index_key(&self, k: IK) -> Vec<u8> {
        k.joined_key()
    }

    fn no_prefix_raw(&self) -> Prefix<Vec<u8>, T, IK> {
        Prefix::with_deserialization_functions(
            self.idx_namespace,
            &[],
            &[],
            |_, _, kv| deserialize_unique_v(kv),
            |_, _, kv| deserialize_unique_v(kv),
        )
    }

    /// returns all items that match this secondary index, always by pk Ascending
    pub fn item(&self, store: &dyn Storage, idx: IK) -> StdResult<Option<Record<T>>> {
        let data = self
            .idx_map
            .may_load(store, idx)?
            .map(|i| (i.pk.into(), i.value));
        Ok(data)
    }
}

// short-cut for simple keys, rather than .prefix(()).range_raw(...)
impl<'a, IK, T, PK> UniqueIndex<'a, IK, T, PK>
where
    T: Serialize + DeserializeOwned + Clone,
    IK: PrimaryKey<'a>,
{
    // I would prefer not to copy code from Prefix, but no other way
    // with lifetimes (create Prefix inside function and return ref = no no)
    pub fn range_raw<'c>(
        &self,
        store: &'c dyn Storage,
        min: Option<Bound<'a, IK>>,
        max: Option<Bound<'a, IK>>,
        order: Order,
    ) -> Box<dyn Iterator<Item = StdResult<Record<T>>> + 'c>
    where
        T: 'c,
    {
        self.no_prefix_raw().range_raw(store, min, max, order)
    }

    pub fn keys_raw<'c>(
        &self,
        store: &'c dyn Storage,
        min: Option<Bound<'a, IK>>,
        max: Option<Bound<'a, IK>>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Vec<u8>> + 'c> {
        self.no_prefix_raw().keys_raw(store, min, max, order)
    }
}

#[cfg(feature = "iterator")]
impl<'a, IK, T, PK> UniqueIndex<'a, IK, T, PK>
where
    PK: PrimaryKey<'a> + KeyDeserialize,
    T: Serialize + DeserializeOwned + Clone,
    IK: PrimaryKey<'a>,
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
        min: Option<PrefixBound<'a, IK::Prefix>>,
        max: Option<PrefixBound<'a, IK::Prefix>>,
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
        min: Option<Bound<'a, IK>>,
        max: Option<Bound<'a, IK>>,
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
        min: Option<Bound<'a, IK>>,
        max: Option<Bound<'a, IK>>,
        order: cosmwasm_std::Order,
    ) -> Box<dyn Iterator<Item = StdResult<PK::Output>> + 'c>
    where
        T: 'c,
        PK::Output: 'static,
    {
        self.no_prefix().keys(store, min, max, order)
    }

    pub fn prefix(&self, p: IK::Prefix) -> Prefix<PK, T, IK::Suffix> {
        Prefix::with_deserialization_functions(
            self.idx_namespace,
            &p.prefix(),
            &[],
            |_, _, kv| deserialize_unique_kv::<PK, _>(kv),
            |_, _, kv| deserialize_unique_v(kv),
        )
    }

    pub fn sub_prefix(&self, p: IK::SubPrefix) -> Prefix<PK, T, IK::SuperSuffix> {
        Prefix::with_deserialization_functions(
            self.idx_namespace,
            &p.prefix(),
            &[],
            |_, _, kv| deserialize_unique_kv::<PK, _>(kv),
            |_, _, kv| deserialize_unique_v(kv),
        )
    }

    fn no_prefix(&self) -> Prefix<PK, T, IK> {
        Prefix::with_deserialization_functions(
            self.idx_namespace,
            &[],
            &[],
            |_, _, kv| deserialize_unique_kv::<PK, _>(kv),
            |_, _, kv| deserialize_unique_v(kv),
        )
    }
}
