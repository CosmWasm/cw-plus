use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;

use cosmwasm_std::{CosmosMsg, Empty};
use cw0::Expiration;

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Cw3HandleMsg<T = Empty>
where
    T: Clone + fmt::Debug + PartialEq + JsonSchema,
{
    Propose {
        title: String,
        description: String,
        msgs: Vec<CosmosMsg<T>>,
        expires: Option<Expiration>,
    },
    Vote {
        proposal_id: u64,
        vote: Vote,
    },
    Execute {
        proposal_id: u64,
    },
    Close {
        proposal_id: u64,
    },
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Vote {
    YES,
    NO,
    ABSTAIN,
    VETO,
}

#[cfg(test)]
mod test {
    use super::*;
    use cosmwasm_std::to_vec;

    #[test]
    fn vote_encoding() {
        let a = Vote::YES;
        let encoded = to_vec(&a).unwrap();
        let json = String::from_utf8_lossy(&encoded).to_string();
        assert_eq!(r#""yes""#, json.as_str());
    }

    #[test]
    fn vote_encoding_embedded() {
        let msg = Cw3HandleMsg::Vote::<Empty> {
            proposal_id: 17,
            vote: Vote::NO,
        };
        let encoded = to_vec(&msg).unwrap();
        let json = String::from_utf8_lossy(&encoded).to_string();
        assert_eq!(r#"{"vote":{"proposal_id":17,"vote":"no"}}"#, json.as_str());
    }
}
