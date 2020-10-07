// This code is intentionally included not in lib.rs
// Most of it will be deleted. But maybe we want to borrow some chunks, so keeping them here.

/// Calculates the raw key prefix for a given namespace as documented
/// in https://github.com/webmaster128/key-namespacing#length-prefixed-keys
pub(crate) fn to_length_prefixed(namespace: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(namespace.len() + 2);
    out.extend_from_slice(&encode_length(namespace));
    out.extend_from_slice(namespace);
    out
}

pub(crate) fn length_prefixed_with_key(namespace: &[u8], key: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(namespace.len() + 2 + key.len());
    out.extend_from_slice(&encode_length(namespace));
    out.extend_from_slice(namespace);
    out.extend_from_slice(key);
    out
}

/// This is equivalent concat(to_length_prefixed_nested(namespaces), key)
/// But more efficient when the intermediate namespaces often must be recalculated
pub(crate) fn namespaces_with_key(namespaces: &[&[u8]], key: &[u8]) -> Vec<u8> {
    let mut size = key.len();
    for &namespace in namespaces {
        size += namespace.len() + 2;
    }

    let mut out = Vec::with_capacity(size);
    for &namespace in namespaces {
        out.extend_from_slice(&encode_length(namespace));
        out.extend_from_slice(namespace);
    }
    out.extend_from_slice(key);
    out
}

// pub(crate) fn decode_length(prefix: [u8; 2]) -> usize {
pub(crate) fn decode_length(prefix: &[u8]) -> usize {
    // TODO: enforce exactly 2 bytes somehow, but usable with slices
    (prefix[0] as usize) * 256 + (prefix[1] as usize)
}

pub(crate) fn get_with_prefix<S: ReadonlyStorage>(
    storage: &S,
    namespace: &[u8],
    key: &[u8],
) -> Option<Vec<u8>> {
    storage.get(&concat(namespace, key))
}

pub(crate) fn set_with_prefix<S: Storage>(
    storage: &mut S,
    namespace: &[u8],
    key: &[u8],
    value: &[u8],
) {
    storage.set(&concat(namespace, key), value);
}

pub(crate) fn remove_with_prefix<S: Storage>(storage: &mut S, namespace: &[u8], key: &[u8]) {
    storage.remove(&concat(namespace, key));
}

#[cfg(test)]
mod legacy_test {
    use super::*;
    use crate::helpers::*;
    use cosmwasm_std::testing::MockStorage;

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