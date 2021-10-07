#![cfg(feature = "iterator")]
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::marker::PhantomData;

use cosmwasm_std::{Order, Record, StdResult, Storage};
use std::ops::Deref;

use crate::de::KeyDeserialize;
use crate::helpers::{namespaces_with_key, nested_namespaces_with_key};
use crate::iter_helpers::{concat, deserialize_kv, deserialize_v, trim};
use crate::{Endian, Prefixer};

/// Bound is used to defines the two ends of a range, more explicit than Option<u8>
/// None means that we don't limit that side of the range at all.
/// Include means we use the given bytes as a limit and *include* anything at that exact key
/// Exclude means we use the given bytes as a limit and *exclude* anything at that exact key
#[derive(Clone, Debug)]
pub enum Bound {
    Inclusive(Vec<u8>),
    Exclusive(Vec<u8>),
}

impl Bound {
    /// Turns optional binary, like Option<CanonicalAddr> into an inclusive bound
    pub fn inclusive<T: Into<Vec<u8>>>(limit: T) -> Self {
        Bound::Inclusive(limit.into())
    }

    /// Turns optional binary, like Option<CanonicalAddr> into an exclusive bound
    pub fn exclusive<T: Into<Vec<u8>>>(limit: T) -> Self {
        Bound::Exclusive(limit.into())
    }

    /// Turns an int, like Option<u32> into an inclusive bound
    pub fn inclusive_int<T: Endian>(limit: T) -> Self {
        Bound::Inclusive(limit.to_be_bytes().into())
    }

    /// Turns an int, like Option<u64> into an exclusive bound
    pub fn exclusive_int<T: Endian>(limit: T) -> Self {
        Bound::Exclusive(limit.to_be_bytes().into())
    }
}

#[derive(Clone, Debug)]
pub enum PrefixBound<'a, K: Prefixer<'a>> {
    Inclusive((K, PhantomData<&'a bool>)),
    Exclusive((K, PhantomData<&'a bool>)),
}

impl<'a, K: Prefixer<'a>> PrefixBound<'a, K> {
    pub fn inclusive<T: Into<K>>(k: T) -> Self {
        Self::Inclusive((k.into(), PhantomData))
    }

    pub fn exclusive<T: Into<K>>(k: T) -> Self {
        Self::Exclusive((k.into(), PhantomData))
    }

    pub fn to_bound(&self) -> Bound {
        match self {
            PrefixBound::Exclusive((k, _)) => Bound::Exclusive(k.joined_prefix()),
            PrefixBound::Inclusive((k, _)) => Bound::Inclusive(k.joined_prefix()),
        }
    }
}

type DeserializeKvFn<K, T> =
    fn(&dyn Storage, &[u8], Record) -> StdResult<(<K as KeyDeserialize>::Output, T)>;

#[allow(dead_code)]
pub fn default_deserializer_v<T: DeserializeOwned>(
    _: &dyn Storage,
    _: &[u8],
    raw: Record,
) -> StdResult<Record<T>> {
    deserialize_v(raw)
}

pub fn default_deserializer_kv<K: KeyDeserialize, T: DeserializeOwned>(
    _: &dyn Storage,
    _: &[u8],
    raw: Record,
) -> StdResult<(K::Output, T)> {
    deserialize_kv::<K, T>(raw)
}

#[derive(Clone)]
pub struct Prefix<K, T>
where
    K: KeyDeserialize,
    T: Serialize + DeserializeOwned,
{
    /// all namespaces prefixes and concatenated with the key
    storage_prefix: Vec<u8>,
    // see https://doc.rust-lang.org/std/marker/struct.PhantomData.html#unused-type-parameters for why this is needed
    data: PhantomData<T>,
    pk_name: Vec<u8>,
    de_fn: DeserializeKvFn<K, T>,
}

impl<K, T> Deref for Prefix<K, T>
where
    K: KeyDeserialize,
    T: Serialize + DeserializeOwned,
{
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        &self.storage_prefix
    }
}

impl<K, T> Prefix<K, T>
where
    K: KeyDeserialize,
    T: Serialize + DeserializeOwned,
{
    pub fn new(top_name: &[u8], sub_names: &[&[u8]]) -> Self {
        Prefix::with_deserialization_function(
            top_name,
            sub_names,
            &[],
            default_deserializer_kv::<K, T>,
        )
    }

    pub fn with_deserialization_function(
        top_name: &[u8],
        sub_names: &[&[u8]],
        pk_name: &[u8],
        de_fn: DeserializeKvFn<K, T>,
    ) -> Self {
        let storage_prefix = nested_namespaces_with_key(&[top_name], sub_names, b"");
        Prefix {
            storage_prefix,
            data: PhantomData,
            pk_name: pk_name.to_vec(),
            de_fn,
        }
    }

    pub fn range<'a>(
        &self,
        store: &'a dyn Storage,
        min: Option<Bound>,
        max: Option<Bound>,
        order: Order,
    ) -> Box<dyn Iterator<Item = StdResult<(K::Output, T)>> + 'a>
    where
        T: 'a,
        K::Output: 'a,
    {
        let de_fn = self.de_fn;
        let pk_name = self.pk_name.clone();
        let mapped = range_with_prefix(store, &self.storage_prefix, min, max, order)
            .map(move |kv| (de_fn)(store, &*pk_name, kv));
        Box::new(mapped)
    }

    pub fn keys<'a>(
        &self,
        store: &'a dyn Storage,
        min: Option<Bound>,
        max: Option<Bound>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Vec<u8>> + 'a> {
        let mapped =
            range_with_prefix(store, &self.storage_prefix, min, max, order).map(|(k, _)| k);
        Box::new(mapped)
    }

    pub fn range_de<'a>(
        &self,
        store: &'a dyn Storage,
        min: Option<Bound>,
        max: Option<Bound>,
        order: Order,
    ) -> Box<dyn Iterator<Item = StdResult<(K::Output, T)>> + 'a>
    where
        T: 'a,
        K::Output: 'static,
    {
        let de_fn = self.de_fn;
        let pk_name = self.pk_name.clone();
        let mapped = range_with_prefix(store, &self.storage_prefix, min, max, order)
            .map(move |kv| (de_fn)(store, &*pk_name, kv));
        Box::new(mapped)
    }

    pub fn keys_de<'a>(
        &self,
        store: &'a dyn Storage,
        min: Option<Bound>,
        max: Option<Bound>,
        order: Order,
    ) -> Box<dyn Iterator<Item = StdResult<K::Output>> + 'a>
    where
        T: 'a,
        K::Output: 'static,
    {
        let de_fn = self.de_fn;
        let pk_name = self.pk_name.clone();
        let mapped = range_with_prefix(store, &self.storage_prefix, min, max, order)
            .map(move |kv| (de_fn)(store, &*pk_name, kv).map(|(k, _)| Ok(k)))
            .flatten();
        Box::new(mapped)
    }
}

pub fn range_with_prefix<'a>(
    storage: &'a dyn Storage,
    namespace: &[u8],
    start: Option<Bound>,
    end: Option<Bound>,
    order: Order,
) -> Box<dyn Iterator<Item = Record> + 'a> {
    let start = calc_start_bound(namespace, start);
    let end = calc_end_bound(namespace, end);

    // get iterator from storage
    let base_iterator = storage.range(Some(&start), Some(&end), order);

    // make a copy for the closure to handle lifetimes safely
    let prefix = namespace.to_vec();
    let mapped = base_iterator.map(move |(k, v)| (trim(&prefix, &k), v));
    Box::new(mapped)
}

fn calc_start_bound(namespace: &[u8], bound: Option<Bound>) -> Vec<u8> {
    match bound {
        None => namespace.to_vec(),
        // this is the natural limits of the underlying Storage
        Some(Bound::Inclusive(limit)) => concat(namespace, &limit),
        Some(Bound::Exclusive(limit)) => concat(namespace, &extend_one_byte(&limit)),
    }
}

fn calc_end_bound(namespace: &[u8], bound: Option<Bound>) -> Vec<u8> {
    match bound {
        None => increment_last_byte(namespace),
        // this is the natural limits of the underlying Storage
        Some(Bound::Exclusive(limit)) => concat(namespace, &limit),
        Some(Bound::Inclusive(limit)) => concat(namespace, &extend_one_byte(&limit)),
    }
}

pub fn namespaced_prefix_range<'a, 'c, K: Prefixer<'a>>(
    storage: &'c dyn Storage,
    namespace: &[u8],
    start: Option<PrefixBound<'a, K>>,
    end: Option<PrefixBound<'a, K>>,
    order: Order,
) -> Box<dyn Iterator<Item = Record> + 'c> {
    let prefix = namespaces_with_key(&[namespace], &[]);
    let start = calc_prefix_start_bound(&prefix, start);
    let end = calc_prefix_end_bound(&prefix, end);

    // get iterator from storage
    let base_iterator = storage.range(Some(&start), Some(&end), order);

    // make a copy for the closure to handle lifetimes safely
    let mapped = base_iterator.map(move |(k, v)| (trim(&prefix, &k), v));
    Box::new(mapped)
}

fn calc_prefix_start_bound<'a, K: Prefixer<'a>>(
    namespace: &[u8],
    bound: Option<PrefixBound<'a, K>>,
) -> Vec<u8> {
    match bound.map(|b| b.to_bound()) {
        None => namespace.to_vec(),
        // this is the natural limits of the underlying Storage
        Some(Bound::Inclusive(limit)) => concat(namespace, &limit),
        Some(Bound::Exclusive(limit)) => concat(namespace, &increment_last_byte(&limit)),
    }
}

fn calc_prefix_end_bound<'a, K: Prefixer<'a>>(
    namespace: &[u8],
    bound: Option<PrefixBound<'a, K>>,
) -> Vec<u8> {
    match bound.map(|b| b.to_bound()) {
        None => increment_last_byte(namespace),
        // this is the natural limits of the underlying Storage
        Some(Bound::Exclusive(limit)) => concat(namespace, &limit),
        Some(Bound::Inclusive(limit)) => concat(namespace, &increment_last_byte(&limit)),
    }
}

fn extend_one_byte(limit: &[u8]) -> Vec<u8> {
    let mut v = limit.to_vec();
    v.push(0);
    v
}

/// Returns a new vec of same length and last byte incremented by one
/// If last bytes are 255, we handle overflow up the chain.
/// If all bytes are 255, this returns wrong data - but that is never possible as a namespace
fn increment_last_byte(input: &[u8]) -> Vec<u8> {
    let mut copy = input.to_vec();
    // zero out all trailing 255, increment first that is not such
    for i in (0..input.len()).rev() {
        if copy[i] == 255 {
            copy[i] = 0;
        } else {
            copy[i] += 1;
            break;
        }
    }
    copy
}

#[cfg(test)]
mod test {
    use super::*;
    use cosmwasm_std::testing::MockStorage;

    #[test]
    fn ensure_proper_range_bounds() {
        let mut store = MockStorage::new();
        // manually create this - not testing nested prefixes here
        let prefix: Prefix<Vec<u8>, u64> = Prefix {
            storage_prefix: b"foo".to_vec(),
            data: PhantomData::<u64>,
            pk_name: vec![],
            de_fn: |_, _, kv| deserialize_kv::<Vec<u8>, u64>(kv),
        };

        // set some data, we care about "foo" prefix
        store.set(b"foobar", b"1");
        store.set(b"foora", b"2");
        store.set(b"foozi", b"3");
        // these shouldn't match
        store.set(b"foply", b"100");
        store.set(b"font", b"200");

        let expected = vec![
            (b"bar".to_vec(), 1u64),
            (b"ra".to_vec(), 2u64),
            (b"zi".to_vec(), 3u64),
        ];
        let expected_reversed: Vec<(Vec<u8>, u64)> = expected.iter().rev().cloned().collect();

        // let's do the basic sanity check
        let res: StdResult<Vec<_>> = prefix.range(&store, None, None, Order::Ascending).collect();
        assert_eq!(&expected, &res.unwrap());
        let res: StdResult<Vec<_>> = prefix
            .range(&store, None, None, Order::Descending)
            .collect();
        assert_eq!(&expected_reversed, &res.unwrap());

        // now let's check some ascending ranges
        let res: StdResult<Vec<_>> = prefix
            .range(
                &store,
                Some(Bound::Inclusive(b"ra".to_vec())),
                None,
                Order::Ascending,
            )
            .collect();
        assert_eq!(&expected[1..], res.unwrap().as_slice());
        // skip excluded
        let res: StdResult<Vec<_>> = prefix
            .range(
                &store,
                Some(Bound::Exclusive(b"ra".to_vec())),
                None,
                Order::Ascending,
            )
            .collect();
        assert_eq!(&expected[2..], res.unwrap().as_slice());
        // if we exclude something a little lower, we get matched
        let res: StdResult<Vec<_>> = prefix
            .range(
                &store,
                Some(Bound::Exclusive(b"r".to_vec())),
                None,
                Order::Ascending,
            )
            .collect();
        assert_eq!(&expected[1..], res.unwrap().as_slice());

        // now let's check some descending ranges
        let res: StdResult<Vec<_>> = prefix
            .range(
                &store,
                None,
                Some(Bound::Inclusive(b"ra".to_vec())),
                Order::Descending,
            )
            .collect();
        assert_eq!(&expected_reversed[1..], res.unwrap().as_slice());
        // skip excluded
        let res: StdResult<Vec<_>> = prefix
            .range(
                &store,
                None,
                Some(Bound::Exclusive(b"ra".to_vec())),
                Order::Descending,
            )
            .collect();
        assert_eq!(&expected_reversed[2..], res.unwrap().as_slice());
        // if we exclude something a little higher, we get matched
        let res: StdResult<Vec<_>> = prefix
            .range(
                &store,
                None,
                Some(Bound::Exclusive(b"rb".to_vec())),
                Order::Descending,
            )
            .collect();
        assert_eq!(&expected_reversed[1..], res.unwrap().as_slice());

        // now test when both sides are set
        let res: StdResult<Vec<_>> = prefix
            .range(
                &store,
                Some(Bound::Inclusive(b"ra".to_vec())),
                Some(Bound::Exclusive(b"zi".to_vec())),
                Order::Ascending,
            )
            .collect();
        assert_eq!(&expected[1..2], res.unwrap().as_slice());
        // and descending
        let res: StdResult<Vec<_>> = prefix
            .range(
                &store,
                Some(Bound::Inclusive(b"ra".to_vec())),
                Some(Bound::Exclusive(b"zi".to_vec())),
                Order::Descending,
            )
            .collect();
        assert_eq!(&expected[1..2], res.unwrap().as_slice());
        // Include both sides
        let res: StdResult<Vec<_>> = prefix
            .range(
                &store,
                Some(Bound::Inclusive(b"ra".to_vec())),
                Some(Bound::Inclusive(b"zi".to_vec())),
                Order::Descending,
            )
            .collect();
        assert_eq!(&expected_reversed[..2], res.unwrap().as_slice());
        // Exclude both sides
        let res: StdResult<Vec<_>> = prefix
            .range(
                &store,
                Some(Bound::Exclusive(b"ra".to_vec())),
                Some(Bound::Exclusive(b"zi".to_vec())),
                Order::Ascending,
            )
            .collect();
        assert_eq!(res.unwrap().as_slice(), &[]);
    }
}
