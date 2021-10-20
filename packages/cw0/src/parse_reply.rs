use thiserror::Error;

use cosmwasm_std::{Binary, Reply};

#[derive(Clone, Debug, PartialEq)]
pub struct MsgInstantiateContractResponse {
    pub contract_address: String,
    pub data: Option<Binary>, // FIXME: Confirm if Option
}

pub fn parse_reply_instantiate_data(
    msg: Reply,
) -> Result<MsgInstantiateContractResponse, ParseReplyError> {
    let id = msg.id;
    let data = msg
        .result
        .into_result()
        .map_err(ParseReplyError::SubMsgFailure)?
        .data
        .ok_or_else(|| ParseReplyError::ParseFailure {
            id,
            err: "Missing reply data".to_owned(),
        })?;
    // Manual protobuf decoding
    let data = data.0;
    println!("reply data 0: {:#?}", data);
    // FIXME: avoid panics
    let (wire_type, data) = data.as_slice().split_at(1);
    println!("reply data 1: {:#?}", data);
    if wire_type[0] != b'\x0a' {
        return Err(ParseReplyError::ParseFailure { id, err: "failed to decode Protobuf message: MsgInstantiateContractResponse.contract_address: invalid wire type".to_owned() });
    }
    let (len, data) = data.split_at(1);
    println!("reply data 2: {:#?}", data);
    let (contract_addr, data) = data.split_at(len[0] as usize);
    println!("reply data 3: {:#?}", data);
    let (wire_type, data) = data.split_at(1);
    if wire_type[0] != b'\x12' {
        return Err(ParseReplyError::ParseFailure { id, err: "failed to decode Protobuf message: MsgInstantiateContractResponse.data: invalid wire type".to_owned() });
    }
    let (len, data) = data.split_at(1);
    println!("reply data 4: {:#?}", data);
    let (data, _rest) = data.split_at(len[0] as usize);
    println!("reply data 5: {:#?}", data);
    let data = if data.is_empty() {
        None
    } else {
        Some(Binary(data.to_vec()))
    };

    let res = MsgInstantiateContractResponse {
        contract_address: String::from_utf8(contract_addr.to_vec())?,
        data,
    };

    Ok(res)
}

#[derive(Error, Debug, PartialEq)]
pub enum ParseReplyError {
    #[error("Failure response from sub-message: {0}")]
    SubMsgFailure(String),

    #[error("Invalid reply from sub-message {id}: {err}")]
    ParseFailure { id: u64, err: String },

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
