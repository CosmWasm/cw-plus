//! This module is an implemention of a namespacing scheme described
//! in https://github.com/webmaster128/key-namespacing#length-prefixed-keys
//!
//! Everything in this file is only responsible for building such keys
//! and is in no way specific to any kind of storage.

use serde::de::DeserializeOwned;
use std::any::type_name;

use crate::keys::Key;

use cosmwasm_std::{
    from_slice, to_vec, Addr, Binary, ContractResult, CustomQuery, QuerierWrapper, QueryRequest,
    StdError, StdResult, SystemResult, WasmQuery,
};

/// may_deserialize parses json bytes from storage (Option), returning Ok(None) if no data present
///
/// value is an odd type, but this is meant to be easy to use with output from storage.get (Option<Vec<u8>>)
/// and value.map(|s| s.as_slice()) seems trickier than &value
pub(crate) fn may_deserialize<T: DeserializeOwned>(
    value: &Option<Vec<u8>>,
) -> StdResult<Option<T>> {
    match value {
        Some(vec) => Ok(Some(from_slice(vec)?)),
        None => Ok(None),
    }
}

/// must_deserialize parses json bytes from storage (Option), returning NotFound error if no data present
pub(crate) fn must_deserialize<T: DeserializeOwned>(value: &Option<Vec<u8>>) -> StdResult<T> {
    match value {
        Some(vec) => from_slice(vec),
        None => Err(StdError::not_found(type_name::<T>())),
    }
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

/// Customization of namespaces_with_key for when
/// there are multiple sets we do not want to combine just to call this
pub(crate) fn nested_namespaces_with_key(
    top_names: &[&[u8]],
    sub_names: &[Key],
    key: &[u8],
) -> Vec<u8> {
    let mut size = key.len();
    for &namespace in top_names {
        size += namespace.len() + 2;
    }
    for namespace in sub_names {
        size += namespace.as_ref().len() + 2;
    }

    let mut out = Vec::with_capacity(size);
    for &namespace in top_names {
        out.extend_from_slice(&encode_length(namespace));
        out.extend_from_slice(namespace);
    }
    for namespace in sub_names {
        out.extend_from_slice(&encode_length(namespace.as_ref()));
        out.extend_from_slice(namespace.as_ref());
    }
    out.extend_from_slice(key);
    out
}

/// Encodes the length of a given namespace as a 2 byte big endian encoded integer
pub(crate) fn encode_length(namespace: &[u8]) -> [u8; 2] {
    if namespace.len() > 0xFFFF {
        panic!("only supports namespaces up to length 0xFFFF")
    }
    let length_bytes = (namespace.len() as u32).to_be_bytes();
    [length_bytes[2], length_bytes[3]]
}

/// Use this in Map/SnapshotMap/etc when you want to provide a QueryRaw helper.
/// This is similar to querier.query(WasmQuery::Raw{}), except it does NOT parse the
/// result, but return a possibly empty Binary to be handled by the calling code.
/// That is essential to handle b"" as None.
pub(crate) fn query_raw<Q: CustomQuery>(
    querier: &QuerierWrapper<Q>,
    contract_addr: Addr,
    key: Binary,
) -> StdResult<Binary> {
    let request: QueryRequest<Q> = WasmQuery::Raw {
        contract_addr: contract_addr.into(),
        key,
    }
    .into();

    let raw = to_vec(&request).map_err(|serialize_err| {
        StdError::generic_err(format!("Serializing QueryRequest: {}", serialize_err))
    })?;
    match querier.raw_query(&raw) {
        SystemResult::Err(system_err) => Err(StdError::generic_err(format!(
            "Querier system error: {}",
            system_err
        ))),
        SystemResult::Ok(ContractResult::Err(contract_err)) => Err(StdError::generic_err(format!(
            "Querier contract error: {}",
            contract_err
        ))),
        SystemResult::Ok(ContractResult::Ok(value)) => Ok(value),
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use cosmwasm_std::{to_vec, StdError};
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Person {
        pub name: String,
        pub age: i32,
    }

    #[test]
    fn encode_length_works() {
        assert_eq!(encode_length(b""), *b"\x00\x00");
        assert_eq!(encode_length(b"a"), *b"\x00\x01");
        assert_eq!(encode_length(b"aa"), *b"\x00\x02");
        assert_eq!(encode_length(b"aaa"), *b"\x00\x03");
        assert_eq!(encode_length(&vec![1; 255]), *b"\x00\xff");
        assert_eq!(encode_length(&vec![1; 256]), *b"\x01\x00");
        assert_eq!(encode_length(&vec![1; 12345]), *b"\x30\x39");
        assert_eq!(encode_length(&vec![1; 65535]), *b"\xff\xff");
    }

    #[test]
    #[should_panic(expected = "only supports namespaces up to length 0xFFFF")]
    fn encode_length_panics_for_large_values() {
        encode_length(&vec![1; 65536]);
    }

    #[test]
    fn may_deserialize_handles_some() {
        let person = Person {
            name: "Maria".to_string(),
            age: 42,
        };
        let value = to_vec(&person).unwrap();

        let may_parse: Option<Person> = may_deserialize(&Some(value)).unwrap();
        assert_eq!(may_parse, Some(person));
    }

    #[test]
    fn may_deserialize_handles_none() {
        let may_parse = may_deserialize::<Person>(&None).unwrap();
        assert_eq!(may_parse, None);
    }

    #[test]
    fn must_deserialize_handles_some() {
        let person = Person {
            name: "Maria".to_string(),
            age: 42,
        };
        let value = to_vec(&person).unwrap();
        let loaded = Some(value);

        let parsed: Person = must_deserialize(&loaded).unwrap();
        assert_eq!(parsed, person);
    }

    #[test]
    fn must_deserialize_handles_none() {
        let parsed = must_deserialize::<Person>(&None);
        match parsed.unwrap_err() {
            StdError::NotFound { kind, .. } => {
                assert_eq!(kind, "cw_storage_plus::helpers::test::Person")
            }
            e => panic!("Unexpected error {}", e),
        }
    }
}
