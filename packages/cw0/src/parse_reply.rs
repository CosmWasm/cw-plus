use thiserror::Error;

use cosmwasm_std::{Binary, Reply};

// Protobuf wire types (https://developers.google.com/protocol-buffers/docs/encoding)
const WIRE_TYPE_LENGTH_DELIMITED: u8 = 2;

#[derive(Clone, Debug, PartialEq)]
pub struct MsgInstantiateContractResponse {
    pub contract_address: String,
    pub data: Option<Binary>, // FIXME: Confirm if Option
}

fn parse_protobuf_string(data: &mut Vec<u8>) -> Result<String, ParseReplyError> {
    if data.is_empty() {
        return Err(ParseReplyError::ParseFailure(
            "failed to decode Protobuf message: string field: message too short".to_owned(),
        ));
    }
    let mut rest_1 = data.split_off(1);
    if data[0] & 0x03 != WIRE_TYPE_LENGTH_DELIMITED {
        return Err(ParseReplyError::ParseFailure(
            "failed to decode Protobuf message: string field: invalid wire type".to_owned(),
        ));
    }
    if rest_1.is_empty() {
        return Err(ParseReplyError::ParseFailure(
            "failed to decode Protobuf message: string field: message too short".to_owned(),
        ));
    }
    let mut rest_2 = rest_1.split_off(1);
    let len = rest_1[0] as usize;
    if rest_2.len() < len {
        return Err(ParseReplyError::ParseFailure(
            "failed to decode Protobuf message: string field: message too short".to_owned(),
        ));
    }
    let rest_3 = rest_2.split_off(len);

    *data = rest_3;
    Ok(String::from_utf8(rest_2)?)
}

fn parse_protobuf_bytes(data: &mut Vec<u8>) -> Result<Option<Binary>, ParseReplyError> {
    if data.is_empty() {
        return Ok(None);
    }
    let mut rest_1 = data.split_off(1);
    if data[0] & 0x03 != WIRE_TYPE_LENGTH_DELIMITED {
        return Err(ParseReplyError::ParseFailure(
            "failed to decode Protobuf message: bytes field: invalid wire type".to_owned(),
        ));
    }
    if rest_1.is_empty() {
        return Err(ParseReplyError::ParseFailure(
            "failed to decode Protobuf message: bytes field: message too short".to_owned(),
        ));
    }
    let mut rest_2 = rest_1.split_off(1);
    let len = rest_1[0] as usize;
    if rest_2.len() < len {
        return Err(ParseReplyError::ParseFailure(
            "failed to decode Protobuf message: bytes field: message too short".to_owned(),
        ));
    }
    let rest_3 = rest_2.split_off(len);

    *data = rest_3;
    Ok(Some(Binary(rest_2.to_vec())))
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
    let contract_addr = parse_protobuf_string(&mut data)?;

    // Parse (optional) data
    let data = parse_protobuf_bytes(&mut data)?;

    let res = MsgInstantiateContractResponse {
        contract_address: contract_addr,
        data,
    };

    Ok(res)
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

    #[test]
    fn parse_reply_instantiate_data_works() {
        let instantiate_reply_data: &str = "Contract #1";
        let instantiate_reply = MsgInstantiateContractResponse {
            contract_address: instantiate_reply_data.to_string(),
            data: vec![1u8, 2, 3, 4],
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
        assert_eq!(
            res,
            super::MsgInstantiateContractResponse {
                contract_address: instantiate_reply_data.into(),
                data: Some(Binary(vec![1u8, 2, 3, 4])),
            }
        );
    }
}
