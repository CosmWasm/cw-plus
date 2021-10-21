use thiserror::Error;

use cosmwasm_std::{Binary, Reply};

// Protobuf wire types (https://developers.google.com/protocol-buffers/docs/encoding)
const WIRE_TYPE_LENGTH_DELIMITED: u8 = 2;

#[derive(Clone, Debug, PartialEq)]
pub struct MsgInstantiateContractResponse {
    pub contract_address: String,
    pub data: Option<Binary>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MsgExecuteContractResponse {
    pub data: Option<Binary>,
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
    if rest_1.is_empty() {
        return Err(ParseReplyError::ParseFailure(format!(
            "failed to decode Protobuf message: field #{}: message too short",
            field_number
        )));
    }
    let mut rest_2 = rest_1.split_off(1);
    let len = rest_1[0] as usize;
    if rest_2.len() < len {
        return Err(ParseReplyError::ParseFailure(format!(
            "failed to decode Protobuf message: field #{}: message too short",
            field_number
        )));
    }
    let rest_3 = rest_2.split_off(len);

    *data = rest_3;
    Ok(rest_2)
}

fn parse_protobuf_string(data: &mut Vec<u8>, field_number: u8) -> Result<String, ParseReplyError> {
    let str_field = parse_protobuf_length_prefixed(data, field_number)?;
    if str_field.is_empty() {
        return Err(ParseReplyError::ParseFailure(format!(
            "failed to decode Protobuf message: string field #{}: message too short",
            field_number
        )));
    }
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
    FromUtf8(#[from] std::string::FromUtf8Error),
}

#[cfg(test)]
mod test {
    use super::*;
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
    fn parse_reply_instantiate_data_works() {
        let contract_addr: &str = "Contract #1";
        for data in [vec![], vec![1u8, 2, 255, 7, 5]] {
            let expected = if data.is_empty() {
                super::MsgInstantiateContractResponse {
                    contract_address: contract_addr.to_string(),
                    data: None,
                }
            } else {
                super::MsgInstantiateContractResponse {
                    contract_address: contract_addr.to_string(),
                    data: Some(Binary(data.clone())),
                }
            };
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
        for data in [vec![], vec![1u8, 2, 3, 127]] {
            let expected = if data.is_empty() {
                super::MsgExecuteContractResponse { data: None }
            } else {
                super::MsgExecuteContractResponse {
                    data: Some(Binary(data.clone())),
                }
            };

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
