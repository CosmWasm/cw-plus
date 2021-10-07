#![cfg(feature = "iterator")]

use serde::de::DeserializeOwned;

use cosmwasm_std::Record;
use cosmwasm_std::{from_slice, StdResult};

use crate::de::KeyDeserialize;
use crate::helpers::encode_length;

#[allow(dead_code)]
pub(crate) fn deserialize_v<T: DeserializeOwned>(kv: Record) -> StdResult<Record<T>> {
    let (k, v) = kv;
    let t = from_slice::<T>(&v)?;
    Ok((k, t))
}

pub(crate) fn deserialize_kv<K: KeyDeserialize, T: DeserializeOwned>(
    kv: Record,
) -> StdResult<(K::Output, T)> {
    let (k, v) = kv;
    let kt = K::from_vec(k)?;
    let vt = from_slice::<T>(&v)?;
    Ok((kt, vt))
}

/// Calculates the raw key prefix for a given namespace as documented
/// in https://github.com/webmaster128/key-namespacing#length-prefixed-keys
#[allow(dead_code)]
pub(crate) fn to_length_prefixed(namespace: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(namespace.len() + 2);
    out.extend_from_slice(&encode_length(namespace));
    out.extend_from_slice(namespace);
    out
}

// TODO: add a check here that it is the real prefix?
#[inline]
pub(crate) fn trim(namespace: &[u8], key: &[u8]) -> Vec<u8> {
    key[namespace.len()..].to_vec()
}

#[inline]
pub(crate) fn concat(namespace: &[u8], key: &[u8]) -> Vec<u8> {
    let mut k = namespace.to_vec();
    k.extend_from_slice(key);
    k
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn to_length_prefixed_works() {
        assert_eq!(to_length_prefixed(b""), b"\x00\x00");
        assert_eq!(to_length_prefixed(b"a"), b"\x00\x01a");
        assert_eq!(to_length_prefixed(b"ab"), b"\x00\x02ab");
        assert_eq!(to_length_prefixed(b"abc"), b"\x00\x03abc");
    }

    #[test]
    fn to_length_prefixed_works_for_long_prefix() {
        let long_namespace1 = vec![0; 256];
        let prefix1 = to_length_prefixed(&long_namespace1);
        assert_eq!(prefix1.len(), 256 + 2);
        assert_eq!(&prefix1[0..2], b"\x01\x00");

        let long_namespace2 = vec![0; 30000];
        let prefix2 = to_length_prefixed(&long_namespace2);
        assert_eq!(prefix2.len(), 30000 + 2);
        assert_eq!(&prefix2[0..2], b"\x75\x30");

        let long_namespace3 = vec![0; 0xFFFF];
        let prefix3 = to_length_prefixed(&long_namespace3);
        assert_eq!(prefix3.len(), 0xFFFF + 2);
        assert_eq!(&prefix3[0..2], b"\xFF\xFF");
    }

    #[test]
    #[should_panic(expected = "only supports namespaces up to length 0xFFFF")]
    fn to_length_prefixed_panics_for_too_long_prefix() {
        let limit = 0xFFFF;
        let long_namespace = vec![0; limit + 1];
        to_length_prefixed(&long_namespace);
    }

    #[test]
    fn to_length_prefixed_calculates_capacity_correctly() {
        // Those tests cannot guarantee the required capacity was calculated correctly before
        // the vector allocation but increase the likelyhood of a proper implementation.

        let key = to_length_prefixed(b"");
        assert_eq!(key.capacity(), key.len());

        let key = to_length_prefixed(b"h");
        assert_eq!(key.capacity(), key.len());

        let key = to_length_prefixed(b"hij");
        assert_eq!(key.capacity(), key.len());
    }
}

// currently disabled tests as they require a bunch of legacy non-sense
// TODO: enable
#[cfg(test)]
#[cfg(not(feature = "iterator"))]
mod namespace_test {
    use super::*;
    use cosmwasm_std::testing::MockStorage;

    #[test]
    fn test_range() {
        let mut storage = MockStorage::new();
        let prefix = to_length_prefixed(b"foo");
        let other_prefix = to_length_prefixed(b"food");

        // set some values in this range
        set_with_prefix(&mut storage, &prefix, b"bar", b"none");
        set_with_prefix(&mut storage, &prefix, b"snowy", b"day");

        // set some values outside this range
        set_with_prefix(&mut storage, &other_prefix, b"moon", b"buggy");

        // ensure we get proper result from prefixed_range iterator
        let mut iter = range_with_prefix(&storage, &prefix, None, None, Order::Descending);
        let first = iter.next().unwrap();
        assert_eq!(first, (b"snowy".to_vec(), b"day".to_vec()));
        let second = iter.next().unwrap();
        assert_eq!(second, (b"bar".to_vec(), b"none".to_vec()));
        assert!(iter.next().is_none());

        // ensure we get raw result from base range
        let iter = storage.range(None, None, Order::Ascending);
        assert_eq!(3, iter.count());

        // foo comes first
        let mut iter = storage.range(None, None, Order::Ascending);
        let first = iter.next().unwrap();
        let expected_key = concat(&prefix, b"bar");
        assert_eq!(first, (expected_key, b"none".to_vec()));
    }

    #[test]
    fn test_range_with_prefix_wrapover() {
        let mut storage = MockStorage::new();
        // if we don't properly wrap over there will be issues here (note 255+1 is used to calculate end)
        let prefix = to_length_prefixed(b"f\xff\xff");
        let other_prefix = to_length_prefixed(b"f\xff\x44");

        // set some values in this range
        set_with_prefix(&mut storage, &prefix, b"bar", b"none");
        set_with_prefix(&mut storage, &prefix, b"snowy", b"day");

        // set some values outside this range
        set_with_prefix(&mut storage, &other_prefix, b"moon", b"buggy");

        // ensure we get proper result from prefixed_range iterator
        let iter = range_with_prefix(&storage, &prefix, None, None, Order::Descending);
        let elements: Vec<Record> = iter.collect();
        assert_eq!(
            elements,
            vec![
                (b"snowy".to_vec(), b"day".to_vec()),
                (b"bar".to_vec(), b"none".to_vec()),
            ]
        );
    }

    #[test]
    fn test_range_with_start_end_set() {
        let mut storage = MockStorage::new();
        // if we don't properly wrap over there will be issues here (note 255+1 is used to calculate end)
        let prefix = to_length_prefixed(b"f\xff\xff");
        let other_prefix = to_length_prefixed(b"f\xff\x44");

        // set some values in this range
        set_with_prefix(&mut storage, &prefix, b"bar", b"none");
        set_with_prefix(&mut storage, &prefix, b"snowy", b"day");

        // set some values outside this range
        set_with_prefix(&mut storage, &other_prefix, b"moon", b"buggy");

        // make sure start and end are applied properly
        let res: Vec<Record> =
            range_with_prefix(&storage, &prefix, Some(b"b"), Some(b"c"), Order::Ascending)
                .collect();
        assert_eq!(res.len(), 1);
        assert_eq!(res[0], (b"bar".to_vec(), b"none".to_vec()));

        // make sure start and end are applied properly
        let res: Vec<Record> = range_with_prefix(
            &storage,
            &prefix,
            Some(b"bas"),
            Some(b"sno"),
            Order::Ascending,
        )
        .collect();
        assert_eq!(res.len(), 0);

        let res: Vec<Record> =
            range_with_prefix(&storage, &prefix, Some(b"ant"), None, Order::Ascending).collect();
        assert_eq!(res.len(), 2);
        assert_eq!(res[0], (b"bar".to_vec(), b"none".to_vec()));
        assert_eq!(res[1], (b"snowy".to_vec(), b"day".to_vec()));
    }

    #[test]
    fn test_namespace_upper_bound() {
        assert_eq!(namespace_upper_bound(b"bob"), b"boc".to_vec());
        assert_eq!(namespace_upper_bound(b"fo\xfe"), b"fo\xff".to_vec());
        assert_eq!(namespace_upper_bound(b"fo\xff"), b"fp\x00".to_vec());
        // multiple \xff roll over
        assert_eq!(
            namespace_upper_bound(b"fo\xff\xff\xff"),
            b"fp\x00\x00\x00".to_vec()
        );
        // \xff not at the end are ignored
        assert_eq!(namespace_upper_bound(b"\xffabc"), b"\xffabd".to_vec());
    }
}
