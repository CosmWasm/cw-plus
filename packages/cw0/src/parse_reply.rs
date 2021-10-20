use prost::Message;
use thiserror::Error;

use cosmwasm_std::Reply;

#[derive(Clone, PartialEq, Message)]
pub struct MsgInstantiateContractResponse {
    #[prost(string, tag = "1")]
    pub contract_address: ::prost::alloc::string::String,
    #[prost(bytes, tag = "2")]
    pub data: ::prost::alloc::vec::Vec<u8>,
}

pub fn parse_reply_instantiate_data(
    msg: Reply,
) -> Result<MsgInstantiateContractResponse, ParseReplyError> {
    let id = msg.id;
    let res: MsgInstantiateContractResponse = Message::decode(
        msg.result
            .into_result()
            .map_err(ParseReplyError::SubMsgFailure)?
            .data
            .ok_or_else(|| ParseReplyError::ParseFailure {
                id,
                err: "Missing reply data".to_owned(),
            })?
            .as_slice(),
    )
    .map_err(|err| ParseReplyError::ParseFailure {
        id,
        err: err.to_string(),
    })?;

    Ok(res)
}

#[derive(Error, Debug, PartialEq)]
pub enum ParseReplyError {
    #[error("Failure response from sub-message: {0}")]
    SubMsgFailure(String),

    #[error("Invalid reply from sub-message {id}: {err}")]
    ParseFailure { id: u64, err: String },
}

#[cfg(test)]
mod test {
    use super::*;
    use cosmwasm_std::{ContractResult, SubMsgExecutionResponse};

    #[test]
    fn parse_reply_instantiate_data_works() {
        let instantiate_reply_data: &str = "Contract #1";
        let instantiate_reply = MsgInstantiateContractResponse {
            contract_address: instantiate_reply_data.to_string(),
            data: vec![],
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
            MsgInstantiateContractResponse {
                contract_address: instantiate_reply_data.into(),
                data: vec![],
            }
        );
    }
}
