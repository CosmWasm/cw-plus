#[cfg(feature="boot")]
use boot_core::ExecuteFns;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_schema::cw_serde;
use cosmwasm_std::CosmosMsg;
use cw_utils::Expiration;

#[cw_serde]
#[cfg_attr(feature="boot", derive(ExecuteFns))]
pub enum Cw3ExecuteMsg<T> {
    Propose {
        title: String,
        description: String,
        msgs: Vec<CosmosMsg<T>>,
        earliest: Option<Expiration>,
        latest: Option<Expiration>,
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

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, JsonSchema, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Vote {
    /// Marks support for the proposal.
    Yes,
    /// Marks opposition to the proposal.
    No,
    /// Marks participation but does not count towards the ratio of support / opposed
    Abstain,
    /// Veto is generally to be treated as a No vote. Some implementations may allow certain
    /// voters to be able to Veto, or them to be counted stronger than No in some way.
    Veto,
}

#[cfg(test)]
mod test {
    use super::*;
    use cosmwasm_std::to_vec;
    use cosmwasm_std::Empty;

    #[test]
    fn vote_encoding() {
        let a = Vote::Yes;
        let encoded = to_vec(&a).unwrap();
        let json = String::from_utf8_lossy(&encoded).to_string();
        assert_eq!(r#""yes""#, json.as_str());
    }

    #[test]
    fn vote_encoding_embedded() {
        let msg = Cw3ExecuteMsg::<Empty>::Vote {
            proposal_id: 17,
            vote: Vote::No,
        };
        let encoded = to_vec(&msg).unwrap();
        let json = String::from_utf8_lossy(&encoded).to_string();
        assert_eq!(r#"{"vote":{"proposal_id":17,"vote":"no"}}"#, json.as_str());
    }
}
