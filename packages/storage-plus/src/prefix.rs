#![cfg(feature = "iterator")]
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::marker::PhantomData;

use cosmwasm_std::{Order, StdResult, Storage, KV};

use crate::helpers::nested_namespaces_with_key;
use crate::iter_helpers::{concat, deserialize_kv, trim};

/// Bound is used to defines the two ends of a range, more explicit than Option<u8>
/// None means that we don't limit that side of the range at all.
/// Include means we use the given bytes as a limit and *include* anything at that exact key
/// Exclude means we use the given bytes as a limit and *exclude* anything at that exact key
#[derive(Copy, Clone, Debug)]
pub enum Bound<'a> {
    Include(&'a [u8]),
    Exclude(&'a [u8]),
    None,
}

pub struct Prefix<T>
where
    T: Serialize + DeserializeOwned,
{
    /// all namespaces prefixes and concatenated with the key
    pub(crate) storage_prefix: Vec<u8>,
    // see https://doc.rust-lang.org/std/marker/struct.PhantomData.html#unused-type-parameters for why this is needed
    data: PhantomData<T>,
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
        &'a self,
        store: &'a S,
        start: Bound<'_>,
        end: Bound<'_>,
        order: Order,
    ) -> Box<dyn Iterator<Item = StdResult<KV<T>>> + 'a> {
        let mapped = range_with_prefix(store, &self.storage_prefix, start, end, order)
            .map(deserialize_kv::<T>);
        Box::new(mapped)
    }
}

pub(crate) fn range_with_prefix<'a, S: Storage>(
    storage: &'a S,
    namespace: &[u8],
    start: Bound<'_>,
    end: Bound<'_>,
    order: Order,
) -> Box<dyn Iterator<Item = KV> + 'a> {
    let start = calc_start_bound(namespace, start, order);
    let end = calc_end_bound(namespace, end, order);

    // get iterator from storage
    let base_iterator = storage.range(Some(&start), Some(&end), order);

    // make a copy for the closure to handle lifetimes safely
    let prefix = namespace.to_vec();
    let mapped = base_iterator.map(move |(k, v)| (trim(&prefix, &k), v));
    Box::new(mapped)
}

// TODO: does order matter here?
fn calc_start_bound(namespace: &[u8], bound: Bound<'_>, _order: Order) -> Vec<u8> {
    match bound {
        Bound::None => namespace.to_vec(),
        // this is the natural limits of the underlying Storage
        Bound::Include(limit) => concat(namespace, limit),
        Bound::Exclude(limit) => concat(namespace, &one_byte_higher(limit)),
    }
}

// TODO: does order matter here?
fn calc_end_bound(namespace: &[u8], bound: Bound<'_>, _order: Order) -> Vec<u8> {
    match bound {
        Bound::None => namespace_upper_bound(namespace),
        // this is the natural limits of the underlying Storage
        Bound::Exclude(limit) => concat(namespace, limit),
        Bound::Include(limit) => concat(namespace, &one_byte_higher(limit)),
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
            (b"foobar".to_vec(), 1u64),
            (b"foora".to_vec(), 2u64),
            (b"foozi".to_vec(), 3u64),
        ];
        let expected_reversed: Vec<(Vec<u8>, u64)> = expected.iter().clone().rev().collect();

        // let's do the basic sanity check
        let res: StdResult<Vec<_>> = prefix
            .range(&store, Bound::None, Bound::None, Order::Ascending)
            .collect();
        assert_eq!(&expected, &res.unwrap());
        let res: StdResult<Vec<_>> = prefix
            .range(&store, Bound::None, Bound::None, Order::Descending)
            .collect();
        assert_eq!(&expected_reversed, &res.unwrap());

        // now let's check some ascending ranges
        let res: StdResult<Vec<_>> = prefix
            .range(
                &store,
                Bound::Include(b"foora"),
                Bound::None,
                Order::Ascending,
            )
            .collect();
        assert_eq!(&expected[1..], &res.unwrap());
        // skip excluded
        let res: StdResult<Vec<_>> = prefix
            .range(
                &store,
                Bound::Exclude(b"foora"),
                Bound::None,
                Order::Ascending,
            )
            .collect();
        assert_eq!(&expected[2..], &res.unwrap());
        // if we exclude something a little lower, we get matched
        let res: StdResult<Vec<_>> = prefix
            .range(
                &store,
                Bound::Exclude(b"foor"),
                Bound::None,
                Order::Ascending,
            )
            .collect();
        assert_eq!(&expected[1..], &res.unwrap());

        // now let's check some descending ranges
        let res: StdResult<Vec<_>> = prefix
            .range(
                &store,
                Bound::None,
                Bound::Include(b"foora"),
                Order::Descending,
            )
            .collect();
        assert_eq!(&expected_reversed[1..], &res.unwrap());
        // skip excluded
        let res: StdResult<Vec<_>> = prefix
            .range(
                &store,
                Bound::None,
                Bound::Exclude(b"foora"),
                Order::Descending,
            )
            .collect();
        assert_eq!(&expected_reversed[2..], &res.unwrap());
        // if we exclude something a little higher, we get matched
        let res: StdResult<Vec<_>> = prefix
            .range(
                &store,
                Bound::None,
                Bound::Exclude(b"foorb"),
                Order::Descending,
            )
            .collect();
        assert_eq!(&expected_reversed[1..], &res.unwrap());

        // now test when both sides are set
        let res: StdResult<Vec<_>> = prefix
            .range(
                &store,
                Bound::Include(b"foora"),
                Bound::Exclude(b"foozi"),
                Order::Ascending,
            )
            .collect();
        assert_eq!(&expected[1..2], &res.unwrap());
        // and descending
        let res: StdResult<Vec<_>> = prefix
            .range(
                &store,
                Bound::Include(b"foora"),
                Bound::Exclude(b"foozi"),
                Order::Descending,
            )
            .collect();
        assert_eq!(&expected[1..2], &res.unwrap());
        // Include both sides
        let res: StdResult<Vec<_>> = prefix
            .range(
                &store,
                Bound::Include(b"foora"),
                Bound::Include(b"foozi"),
                Order::Descending,
            )
            .collect();
        assert_eq!(&expected_reversed[..2], &res.unwrap());
        // Exclude both sides
        let res: StdResult<Vec<_>> = prefix
            .range(
                &store,
                Bound::Exclude(b"foora"),
                Bound::Exclude(b"foozi"),
                Order::Ascending,
            )
            .collect();
        assert_eq!(&[], &res.unwrap());
    }
}
