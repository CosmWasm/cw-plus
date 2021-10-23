use thiserror::Error;

use cosmwasm_std::{Binary, Reply};

// Protobuf wire types (https://developers.google.com/protocol-buffers/docs/encoding)
const WIRE_TYPE_LENGTH_DELIMITED: u8 = 2;
// Up to 9 bytes of varints as a practical limit (https://github.com/multiformats/unsigned-varint#practical-maximum-of-9-bytes-for-security)
const VARINT_MAX_BYTES: usize = 9;

#[derive(Clone, Debug, PartialEq)]
pub struct MsgInstantiateContractResponse {
    pub contract_address: String,
    pub data: Option<Binary>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MsgExecuteContractResponse {
    pub data: Option<Binary>,
}

/// Base128 varint decoding.
/// The remaining of the data is kept in the data parameter.
fn parse_protobuf_varint(data: &mut Vec<u8>, field_number: u8) -> Result<usize, ParseReplyError> {
    let data_len = data.len();
    let mut len: u64 = 0;
    let mut i = 0;
    while i < VARINT_MAX_BYTES {
        if data_len == i {
            return Err(ParseReplyError::ParseFailure(format!(
                "failed to decode Protobuf message: field #{}: varint data too short",
                field_number
            )));
        }
        len >>= 7;
        len += ((data[i] & 0x7f) as u64) << 56;
        if data[i] & 0x80 == 0 {
            break;
        }
        i += 1;
    }
    if i == VARINT_MAX_BYTES {
        return Err(ParseReplyError::ParseFailure(format!(
            "failed to decode Protobuf message: field #{}: varint data too long",
            field_number
        )));
    }
    i += 1;
    len >>= (VARINT_MAX_BYTES - i) * 7;
    *data = data.split_off(i);

    Ok(len as usize) // Gently fall back to the arch's max addressable size
}

/// Helper function to parse length-prefixed protobuf fields.
/// The remaining of the data is kept in the data parameter.
fn parse_protobuf_length_prefixed(
    data: &mut Vec<u8>,
    field_number: u8,
) -> Result<Vec<u8>, ParseReplyError> {
    if data.is_empty() {
        return Ok(vec![]);
    };
    let mut rest_1 = data.split_off(1);
    let wire_type = data[0] & 0b11;
    let field = data[0] >> 3;

    if field != field_number {
        return Err(ParseReplyError::ParseFailure(format!(
            "failed to decode Protobuf message: invalid field #{} for field #{}",
            field, field_number
        )));
    }
    if wire_type != WIRE_TYPE_LENGTH_DELIMITED {
        return Err(ParseReplyError::ParseFailure(format!(
            "failed to decode Protobuf message: field #{}: invalid wire type {}",
            field_number, wire_type
        )));
    }

    let len = parse_protobuf_varint(&mut rest_1, field_number)?;
    if rest_1.len() < len {
        return Err(ParseReplyError::ParseFailure(format!(
            "failed to decode Protobuf message: field #{}: message too short",
            field_number
        )));
    }
    *data = rest_1.split_off(len);

    Ok(rest_1)
}

fn parse_protobuf_string(data: &mut Vec<u8>, field_number: u8) -> Result<String, ParseReplyError> {
    if data.is_empty() {
        return Err(ParseReplyError::ParseFailure(format!(
            "failed to decode Protobuf message: string field #{}: message too short",
            field_number
        )));
    }
    let str_field = parse_protobuf_length_prefixed(data, field_number)?;
    Ok(String::from_utf8(str_field)?)
}

fn parse_protobuf_bytes(
    data: &mut Vec<u8>,
    field_number: u8,
) -> Result<Option<Binary>, ParseReplyError> {
    let bytes_field = parse_protobuf_length_prefixed(data, field_number)?;
    if bytes_field.is_empty() {
        Ok(None)
    } else {
        Ok(Some(Binary(bytes_field)))
    }
}

pub fn parse_reply_instantiate_data(
    msg: Reply,
) -> Result<MsgInstantiateContractResponse, ParseReplyError> {
    let data = msg
        .result
        .into_result()
        .map_err(ParseReplyError::SubMsgFailure)?
        .data
        .ok_or_else(|| ParseReplyError::ParseFailure("Missing reply data".to_owned()))?;
    // Manual protobuf decoding
    let mut data = data.0;
    // Parse contract addr
    let contract_addr = parse_protobuf_string(&mut data, 1)?;

    // Parse (optional) data
    let data = parse_protobuf_bytes(&mut data, 2)?;

    Ok(MsgInstantiateContractResponse {
        contract_address: contract_addr,
        data,
    })
}

pub fn parse_reply_execute_data(msg: Reply) -> Result<MsgExecuteContractResponse, ParseReplyError> {
    let data = msg
        .result
        .into_result()
        .map_err(ParseReplyError::SubMsgFailure)?
        .data
        .ok_or_else(|| ParseReplyError::ParseFailure("Missing reply data".to_owned()))?;
    // Manual protobuf decoding
    let mut data = data.0;
    // Parse (optional) data
    let data = parse_protobuf_bytes(&mut data, 1)?;

    Ok(MsgExecuteContractResponse { data })
}

#[derive(Error, Debug, PartialEq)]
pub enum ParseReplyError {
    #[error("Failure response from sub-message: {0}")]
    SubMsgFailure(String),

    #[error("Invalid reply from sub-message: {0}")]
    ParseFailure(String),

    #[error("Error occurred while converting from UTF-8")]
    BrokenUtf8(#[from] std::string::FromUtf8Error),
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::parse_reply::ParseReplyError::{BrokenUtf8, ParseFailure};
    use cosmwasm_std::{ContractResult, SubMsgExecutionResponse};
    use prost::Message;

    #[derive(Clone, PartialEq, Message)]
    pub struct MsgInstantiateContractResponse {
        #[prost(string, tag = "1")]
        pub contract_address: ::prost::alloc::string::String,
        #[prost(bytes, tag = "2")]
        pub data: ::prost::alloc::vec::Vec<u8>,
    }

    #[derive(Clone, PartialEq, Message)]
    pub struct MsgExecuteContractResponse {
        #[prost(bytes, tag = "1")]
        pub data: ::prost::alloc::vec::Vec<u8>,
    }

    #[test]
    fn parse_protobuf_varint_tests() {
        let field_number = 1;
        // Single-byte varint works
        let mut data = b"\x0a".to_vec();
        let len = parse_protobuf_varint(&mut data, field_number).unwrap();
        assert_eq!(len, 10);

        // Rest is returned
        let mut data = b"\x0a\x0b".to_vec();
        let len = parse_protobuf_varint(&mut data, field_number).unwrap();
        assert_eq!(len, 10);
        assert_eq!(data, b"\x0b".to_vec());

        // Multi-byte varint works
        // 300 % 128 = 44. 44 + 128 = 172 (0xac) (1st byte)
        // 300 / 128 = 2 (x02) (2nd byte)
        let mut data = b"\xac\x02".to_vec();
        let len = parse_protobuf_varint(&mut data, field_number).unwrap();
        assert_eq!(len, 300);

        // Rest is returned
        let mut data = b"\xac\x02\x0c".to_vec();
        let len = parse_protobuf_varint(&mut data, field_number).unwrap();
        assert_eq!(len, 300);
        assert_eq!(data, b"\x0c".to_vec());

        // varint data too short (Empty varint)
        let mut data = vec![];
        let err = parse_protobuf_varint(&mut data, field_number).unwrap_err();
        assert!(matches!(err, ParseFailure(..)));

        // varint data too short (Incomplete varint)
        let mut data = b"\x80".to_vec();
        let err = parse_protobuf_varint(&mut data, field_number).unwrap_err();
        assert!(matches!(err, ParseFailure(..)));

        // varint data too long
        let mut data = b"\x80\x81\x82\x83\x84\x83\x82\x81\x80".to_vec();
        let err = parse_protobuf_varint(&mut data, field_number).unwrap_err();
        assert!(matches!(err, ParseFailure(..)));
    }

    #[test]
    fn parse_protobuf_length_prefixed_tests() {
        let field_number = 1;
        // Single-byte length-prefixed works
        let mut data = b"\x0a\x03abc".to_vec();
        let res = parse_protobuf_length_prefixed(&mut data, field_number).unwrap();
        assert_eq!(res, b"abc".to_vec());
        assert_eq!(data, vec![0u8; 0]);

        // Rest is returned
        let mut data = b"\x0a\x03abcd".to_vec();
        let res = parse_protobuf_length_prefixed(&mut data, field_number).unwrap();
        assert_eq!(res, b"abc".to_vec());
        assert_eq!(data, b"d".to_vec());

        // Multi-byte length-prefixed works
        let mut data = [b"\x0a\xac\x02", vec![65u8; 300].as_slice()]
            .concat()
            .to_vec();
        let res = parse_protobuf_length_prefixed(&mut data, field_number).unwrap();
        assert_eq!(res, vec![65u8; 300]);
        assert_eq!(data, vec![0u8; 0]);

        // Rest is returned
        let mut data = [b"\x0a\xac\x02", vec![65u8; 300].as_slice(), b"rest"]
            .concat()
            .to_vec();
        let res = parse_protobuf_length_prefixed(&mut data, field_number).unwrap();
        assert_eq!(res, vec![65u8; 300]);
        assert_eq!(data, b"rest");

        // message too short
        let mut data = b"\x0a\x01".to_vec();
        let field_number = 1;
        let err = parse_protobuf_length_prefixed(&mut data, field_number).unwrap_err();
        assert!(matches!(err, ParseFailure(..)));

        // invalid wire type
        let mut data = b"\x0b\x01a".to_vec();
        let err = parse_protobuf_length_prefixed(&mut data, field_number).unwrap_err();
        assert!(matches!(err, ParseFailure(..)));

        // invalid field number
        let field_number = 2;
        let mut data = b"\x0a\x01a".to_vec();
        let err = parse_protobuf_length_prefixed(&mut data, field_number).unwrap_err();
        assert!(matches!(err, ParseFailure(..)));
    }

    #[test]
    fn parse_protobuf_bytes_works() {
        for (i, (mut data, field_number, expected, rest)) in (1..).zip([
            (b"\x0a\x00".to_vec(), 1, None, vec![0u8; 0]),
            (
                b"\x0a\x01a".to_vec(),
                1,
                Some(Binary(b"a".to_vec())),
                vec![0u8; 0],
            ),
            (
                b"\x0a\x06testf1".to_vec(),
                1,
                Some(Binary(b"testf1".to_vec())),
                vec![0u8; 0],
            ),
            (
                b"\x12\x09testingf2".to_vec(),
                2,
                Some(Binary(b"testingf2".to_vec())),
                vec![0u8; 0],
            ),
            (
                b"\x0a\x04test_remainder".to_vec(),
                1,
                Some(Binary(b"test".to_vec())),
                b"_remainder".to_vec(),
            ),
        ]) {
            let res = parse_protobuf_bytes(&mut data, field_number).unwrap();
            assert_eq!(res, expected, "test #{}", i);
            assert_eq!(data, rest, "test #{}", i);
        }
    }

    #[test]
    fn parse_protobuf_string_works() {
        for (i, (mut data, field_number, expected, rest)) in (1..).zip([
            (b"\x0a\x00".to_vec(), 1, "", vec![0u8; 0]),
            (b"\x0a\x01a".to_vec(), 1, "a", vec![0u8; 0]),
            (b"\x0a\x06testf1".to_vec(), 1, "testf1", vec![0u8; 0]),
            (b"\x12\x09testingf2".to_vec(), 2, "testingf2", vec![0u8; 0]),
            (
                b"\x0a\x04test_remainder".to_vec(),
                1,
                "test",
                b"_remainder".to_vec(),
            ),
        ]) {
            let res = parse_protobuf_string(&mut data, field_number).unwrap();
            assert_eq!(res, expected, "test #{}", i);
            assert_eq!(data, rest, "test #{}", i);
        }
    }

    #[test]
    fn parse_protobuf_string_errs() {
        // Correct for reference
        let field_number = 1;
        let mut data = b"\x0a\x01a".to_vec();
        let res = parse_protobuf_string(&mut data, field_number).unwrap();
        assert_eq!(res, "a".to_string());

        // string message too short. Non-optional string
        let mut data = vec![];
        let err = parse_protobuf_string(&mut data, field_number).unwrap_err();
        assert!(matches!(err, ParseFailure(..)));

        // FromUtf8Error
        let field_number = 1;
        let mut data = b"\x0a\x04abc\xd3".to_vec();
        let err = parse_protobuf_string(&mut data, field_number).unwrap_err();
        assert!(matches!(err, BrokenUtf8(..)));
    }

    #[test]
    fn parse_reply_instantiate_data_works() {
        let contract_addr: &str = "Contract #1";
        for (data, expected) in [
            (
                vec![],
                super::MsgInstantiateContractResponse {
                    contract_address: contract_addr.to_string(),
                    data: None,
                },
            ),
            (
                vec![1u8, 2, 255, 7, 5],
                super::MsgInstantiateContractResponse {
                    contract_address: contract_addr.to_string(),
                    data: Some(Binary(vec![1u8, 2, 255, 7, 5])),
                },
            ),
            (
                vec![1u8; 127],
                super::MsgInstantiateContractResponse {
                    contract_address: contract_addr.to_string(),
                    data: Some(Binary(vec![1u8; 127])),
                },
            ),
            (
                vec![2u8; 128],
                super::MsgInstantiateContractResponse {
                    contract_address: contract_addr.to_string(),
                    data: Some(Binary(vec![2u8; 128])),
                },
            ),
            (
                vec![3u8; 257],
                super::MsgInstantiateContractResponse {
                    contract_address: contract_addr.to_string(),
                    data: Some(Binary(vec![3u8; 257])),
                },
            ),
        ] {
            let instantiate_reply = MsgInstantiateContractResponse {
                contract_address: contract_addr.to_string(),
                data,
            };
            let mut encoded_instantiate_reply =
                Vec::<u8>::with_capacity(instantiate_reply.encoded_len());
            // The data must encode successfully
            instantiate_reply
                .encode(&mut encoded_instantiate_reply)
                .unwrap();

            // Build reply message
            let msg = Reply {
                id: 1,
                result: ContractResult::Ok(SubMsgExecutionResponse {
                    events: vec![],
                    data: Some(encoded_instantiate_reply.into()),
                }),
            };

            let res = parse_reply_instantiate_data(msg).unwrap();
            assert_eq!(res, expected);
        }
    }

    #[test]
    fn parse_reply_execute_data_works() {
        for (data, expected) in [
            (vec![], super::MsgExecuteContractResponse { data: None }),
            (
                vec![1u8, 2, 3, 127, 15],
                super::MsgExecuteContractResponse {
                    data: Some(Binary(vec![1u8, 2, 3, 127, 15])),
                },
            ),
            (
                vec![0u8; 255],
                super::MsgExecuteContractResponse {
                    data: Some(Binary(vec![0u8; 255])),
                },
            ),
            (
                vec![1u8; 256],
                super::MsgExecuteContractResponse {
                    data: Some(Binary(vec![1u8; 256])),
                },
            ),
            (
                vec![2u8; 32769],
                super::MsgExecuteContractResponse {
                    data: Some(Binary(vec![2u8; 32769])),
                },
            ),
        ] {
            let execute_reply = MsgExecuteContractResponse { data };
            let mut encoded_execute_reply = Vec::<u8>::with_capacity(execute_reply.encoded_len());
            // The data must encode successfully
            execute_reply.encode(&mut encoded_execute_reply).unwrap();

            // Build reply message
            let msg = Reply {
                id: 1,
                result: ContractResult::Ok(SubMsgExecutionResponse {
                    events: vec![],
                    data: Some(encoded_execute_reply.into()),
                }),
            };

            let res = parse_reply_execute_data(msg).unwrap();

            assert_eq!(res, expected);
        }
    }
}