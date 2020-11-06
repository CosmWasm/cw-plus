// This code is intentionally included not in lib.rs
// Most of it will be deleted. But maybe we want to borrow some chunks, so keeping them here.

/// Calculates the raw key prefix for a given nested namespace
/// as documented in https://github.com/webmaster128/key-namespacing#nesting
pub(crate) fn to_length_prefixed_nested(namespaces: &[&[u8]]) -> Vec<u8> {
    let mut size = 0;
    for &namespace in namespaces {
        size += namespace.len() + 2;
    }

    let mut out = Vec::with_capacity(size);
    for &namespace in namespaces {
        out.extend_from_slice(&encode_length(namespace));
        out.extend_from_slice(namespace);
    }
    out
}

pub(crate) fn length_prefixed_with_key(namespace: &[u8], key: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(namespace.len() + 2 + key.len());
    out.extend_from_slice(&encode_length(namespace));
    out.extend_from_slice(namespace);
    out.extend_from_slice(key);
    out
}

pub(crate) fn get_with_prefix<S: ReadonlyStorage>(
    storage: &dyn Storage,
    namespace: &[u8],
    key: &[u8],
) -> Option<Vec<u8>> {
    storage.get(&concat(namespace, key))
}

pub(crate) fn set_with_prefix(
    storage: &mut dyn Storage,
    namespace: &[u8],
    key: &[u8],
    value: &[u8],
) {
    storage.set(&concat(namespace, key), value);
}

pub(crate) fn remove_with_prefix(storage: &mut dyn Storage, namespace: &[u8], key: &[u8]) {
    storage.remove(&concat(namespace, key));
}

#[cfg(test)]
mod legacy_test {
    use super::*;
    use crate::helpers::*;
    use cosmwasm_std::testing::MockStorage;

    #[test]
    fn to_length_prefixed_nested_works() {
        assert_eq!(to_length_prefixed_nested(&[]), b"");
        assert_eq!(to_length_prefixed_nested(&[b""]), b"\x00\x00");
        assert_eq!(to_length_prefixed_nested(&[b"", b""]), b"\x00\x00\x00\x00");

        assert_eq!(to_length_prefixed_nested(&[b"a"]), b"\x00\x01a");
        assert_eq!(
            to_length_prefixed_nested(&[b"a", b"ab"]),
            b"\x00\x01a\x00\x02ab"
        );
        assert_eq!(
            to_length_prefixed_nested(&[b"a", b"ab", b"abc"]),
            b"\x00\x01a\x00\x02ab\x00\x03abc"
        );
    }

    #[test]
    fn to_length_prefixed_nested_allows_many_long_namespaces() {
        // The 0xFFFF limit is for each namespace, not for the combination of them

        let long_namespace1 = vec![0xaa; 0xFFFD];
        let long_namespace2 = vec![0xbb; 0xFFFE];
        let long_namespace3 = vec![0xcc; 0xFFFF];

        let prefix =
            to_length_prefixed_nested(&[&long_namespace1, &long_namespace2, &long_namespace3]);
        assert_eq!(&prefix[0..2], b"\xFF\xFD");
        assert_eq!(&prefix[2..(2 + 0xFFFD)], long_namespace1.as_slice());
        assert_eq!(&prefix[(2 + 0xFFFD)..(2 + 0xFFFD + 2)], b"\xFF\xFe");
        assert_eq!(
            &prefix[(2 + 0xFFFD + 2)..(2 + 0xFFFD + 2 + 0xFFFE)],
            long_namespace2.as_slice()
        );
        assert_eq!(
            &prefix[(2 + 0xFFFD + 2 + 0xFFFE)..(2 + 0xFFFD + 2 + 0xFFFE + 2)],
            b"\xFF\xFf"
        );
        assert_eq!(
            &prefix[(2 + 0xFFFD + 2 + 0xFFFE + 2)..(2 + 0xFFFD + 2 + 0xFFFE + 2 + 0xFFFF)],
            long_namespace3.as_slice()
        );
    }

    #[test]
    fn to_length_prefixed_nested_calculates_capacity_correctly() {
        // Those tests cannot guarantee the required capacity was calculated correctly before
        // the vector allocation but increase the likelyhood of a proper implementation.

        let key = to_length_prefixed_nested(&[]);
        assert_eq!(key.capacity(), key.len());

        let key = to_length_prefixed_nested(&[b""]);
        assert_eq!(key.capacity(), key.len());

        let key = to_length_prefixed_nested(&[b"a"]);
        assert_eq!(key.capacity(), key.len());

        let key = to_length_prefixed_nested(&[b"a", b"bc"]);
        assert_eq!(key.capacity(), key.len());

        let key = to_length_prefixed_nested(&[b"a", b"bc", b"def"]);
        assert_eq!(key.capacity(), key.len());
    }


    #[test]
    fn prefix_get_set() {
        let mut storage = MockStorage::new();
        let prefix = to_length_prefixed(b"foo");

        set_with_prefix(&mut storage, &prefix, b"bar", b"gotcha");
        let rfoo = get_with_prefix(&storage, &prefix, b"bar");
        assert_eq!(rfoo, Some(b"gotcha".to_vec()));

        // no collisions with other prefixes
        let other_prefix = to_length_prefixed(b"fo");
        let collision = get_with_prefix(&storage, &other_prefix, b"obar");
        assert_eq!(collision, None);
    }

}