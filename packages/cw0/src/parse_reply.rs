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
        len += ((data[i] & 0x7f) as u64) << (i * 7);
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
    *data = data[i + 1..].to_owned();

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
    parse_instantiate_response_data(&data.0)
}

pub fn parse_reply_execute_data(msg: Reply) -> Result<MsgExecuteContractResponse, ParseReplyError> {
    let data = msg
        .result
        .into_result()
        .map_err(ParseReplyError::SubMsgFailure)?
        .data
        .ok_or_else(|| ParseReplyError::ParseFailure("Missing reply data".to_owned()))?;
    parse_execute_response_data(&data.0)
}

pub fn parse_instantiate_response_data(
    data: &[u8],
) -> Result<MsgInstantiateContractResponse, ParseReplyError> {
    // Manual protobuf decoding
    let mut data = data.to_vec();
    // Parse contract addr
    let contract_addr = parse_protobuf_string(&mut data, 1)?;

    // Parse (optional) data
    let data = parse_protobuf_bytes(&mut data, 2)?;

    Ok(MsgInstantiateContractResponse {
        contract_address: contract_addr,
        data,
    })
}

pub fn parse_execute_response_data(
    data: &[u8],
) -> Result<MsgExecuteContractResponse, ParseReplyError> {
    // Manual protobuf decoding
    let mut data = data.to_vec();
    let inner_data = parse_protobuf_bytes(&mut data, 1)?;

    Ok(MsgExecuteContractResponse { data: inner_data })
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
    use std::str::from_utf8;

    fn encode_bytes(data: &[u8]) -> Vec<u8> {
        #[derive(Clone, PartialEq, Message)]
        struct ProtobufBytes {
            #[prost(bytes, tag = "1")]
            pub data: Vec<u8>,
        }

        let data = ProtobufBytes {
            data: data.to_vec(),
        };
        let mut encoded_data = Vec::<u8>::with_capacity(data.encoded_len());
        data.encode(&mut encoded_data).unwrap();

        encoded_data
    }

    fn encode_string(data: &str) -> Vec<u8> {
        #[derive(Clone, PartialEq, Message)]
        struct ProtobufString {
            #[prost(string, tag = "1")]
            pub data: String,
        }

        let data = ProtobufString {
            data: data.to_string(),
        };
        let mut encoded_data = Vec::<u8>::with_capacity(data.encoded_len());
        data.encode(&mut encoded_data).unwrap();

        encoded_data
    }

    #[derive(Clone, PartialEq, Message)]
    struct MsgInstantiateContractResponse {
        #[prost(string, tag = "1")]
        pub contract_address: ::prost::alloc::string::String,
        #[prost(bytes, tag = "2")]
        pub data: ::prost::alloc::vec::Vec<u8>,
    }

    #[derive(Clone, PartialEq, Message)]
    struct MsgExecuteContractResponse {
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
        let field_number = 1;

        // Empty works
        let data = vec![];
        let mut encoded_data = encode_bytes(&data);

        let res = parse_protobuf_bytes(&mut encoded_data, field_number).unwrap();
        assert_eq!(res, None);

        // Simple works
        let data = b"test".to_vec();
        let mut encoded_data = encode_bytes(&data);

        let res = parse_protobuf_bytes(&mut encoded_data, field_number).unwrap();
        assert_eq!(res, Some(Binary(data)));

        // Large works
        let data = vec![0x40; 300];
        let mut encoded_data = encode_bytes(&data);

        let res = parse_protobuf_bytes(&mut encoded_data, field_number).unwrap();
        assert_eq!(res, Some(Binary(data)));

        // Field number works
        let field_number = 5;
        let data = b"test field 5".to_vec();
        let mut encoded_data = encode_bytes(&data);
        encoded_data[0] = (field_number << 3) + WIRE_TYPE_LENGTH_DELIMITED;

        let res = parse_protobuf_bytes(&mut encoded_data, field_number).unwrap();
        assert_eq!(res, Some(Binary(data)));

        // Remainder is kept
        let field_number = 1;
        let test_len: usize = 4;
        let data = b"test_remainder".to_vec();
        let mut encoded_data = encode_bytes(&data);
        encoded_data[1] = test_len as u8;

        let res = parse_protobuf_bytes(&mut encoded_data, field_number).unwrap();
        assert_eq!(res, Some(Binary(data[..test_len].to_owned())));
        assert_eq!(encoded_data, data[test_len..].to_owned());
    }

    #[test]
    fn parse_protobuf_string_tests() {
        let field_number = 1;

        // Empty works
        let data = "";
        let mut encoded_data = encode_string(data);

        let res = parse_protobuf_string(&mut encoded_data, field_number).unwrap();
        assert_eq!(res, data);

        // Simple works
        let data = "test";
        let mut encoded_data = encode_string(data);

        let res = parse_protobuf_string(&mut encoded_data, field_number).unwrap();
        assert_eq!(res, data);

        // Large works
        let data = vec![0x40; 300];
        let str_data = from_utf8(data.as_slice()).unwrap();
        let mut encoded_data = encode_string(str_data);

        let res = parse_protobuf_string(&mut encoded_data, field_number).unwrap();
        assert_eq!(res, str_data);

        // Field number works
        let field_number = 5;
        let data = "test field 5";
        let mut encoded_data = encode_string(data);
        encoded_data[0] = (field_number << 3) + WIRE_TYPE_LENGTH_DELIMITED;

        let res = parse_protobuf_string(&mut encoded_data, field_number).unwrap();
        assert_eq!(res, data);

        // Remainder is kept
        let field_number = 1;
        let test_len: usize = 4;
        let data = "test_remainder";
        let mut encoded_data = encode_string(data);
        encoded_data[1] = test_len as u8;

        let res = parse_protobuf_string(&mut encoded_data, field_number).unwrap();
        assert_eq!(res, data[..test_len]);
        assert_eq!(encoded_data, data[test_len..].as_bytes());

        // Broken utf-8 errs
        let field_number = 1;
        let data = "test_X";
        let mut encoded_data = encode_string(data);
        let encoded_len = encoded_data.len();
        encoded_data[encoded_len - 1] = 0xd3;
        let err = parse_protobuf_string(&mut encoded_data, field_number).unwrap_err();
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
