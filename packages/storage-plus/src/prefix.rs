#![cfg(feature = "iterator")]
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::marker::PhantomData;

use cosmwasm_std::{Order, StdResult, Storage, KV};
use std::ops::Deref;

use crate::helpers::nested_namespaces_with_key;
use crate::iter_helpers::{concat, deserialize_kv, trim};
use crate::Endian;

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

#[derive(Debug, Clone)]
pub struct Prefix<T>
where
    T: Serialize + DeserializeOwned,
{
    /// all namespaces prefixes and concatenated with the key
    storage_prefix: Vec<u8>,
    // see https://doc.rust-lang.org/std/marker/struct.PhantomData.html#unused-type-parameters for why this is needed
    data: PhantomData<T>,
}

impl<T> Deref for Prefix<T>
where
    T: Serialize + DeserializeOwned,
{
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        &self.storage_prefix
    }
}

impl<T> Prefix<T>
where
    T: Serialize + DeserializeOwned,
{
    pub fn new(top_name: &[u8], sub_names: &[&[u8]]) -> Self {
        // FIXME: we can use a custom function here, probably make this cleaner
        let storage_prefix = nested_namespaces_with_key(&[top_name], sub_names, b"");
        Prefix {
            storage_prefix,
            data: PhantomData,
        }
    }

    pub fn range<'a, S: Storage>(
        &self,
        store: &'a S,
        min: Option<Bound>,
        max: Option<Bound>,
        order: Order,
    ) -> Box<dyn Iterator<Item = StdResult<KV<T>>> + 'a>
    where
        T: 'a,
    {
        let mapped = range_with_prefix(store, &self.storage_prefix, min, max, order)
            .map(deserialize_kv::<T>);
        Box::new(mapped)
    }
}

pub(crate) fn range_with_prefix<'a, S: Storage>(
    storage: &'a S,
    namespace: &[u8],
    start: Option<Bound>,
    end: Option<Bound>,
    order: Order,
) -> Box<dyn Iterator<Item = KV> + 'a> {
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
        Some(Bound::Exclusive(limit)) => concat(namespace, &one_byte_higher(&limit)),
    }
}

fn calc_end_bound(namespace: &[u8], bound: Option<Bound>) -> Vec<u8> {
    match bound {
        None => namespace_upper_bound(namespace),
        // this is the natural limits of the underlying Storage
        Some(Bound::Exclusive(limit)) => concat(namespace, &limit),
        Some(Bound::Inclusive(limit)) => concat(namespace, &one_byte_higher(&limit)),
    }
}

fn one_byte_higher(limit: &[u8]) -> Vec<u8> {
    let mut v = limit.to_vec();
    v.push(0);
    v
}

/// Returns a new vec of same length and last byte incremented by one
/// If last bytes are 255, we handle overflow up the chain.
/// If all bytes are 255, this returns wrong data - but that is never possible as a namespace
fn namespace_upper_bound(input: &[u8]) -> Vec<u8> {
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
        let prefix = Prefix {
            storage_prefix: b"foo".to_vec(),
            data: PhantomData::<u64>,
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
