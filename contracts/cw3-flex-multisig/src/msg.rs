use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error::ContractError;
use cosmwasm_std::{CosmosMsg, Decimal, Empty};
use cw0::{Duration, Expiration};
use cw3::{ThresholdResponse, Vote};
use cw4::MemberChangedHookMsg;

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct InstantiateMsg {
    // this is the group contract that contains the member list
    pub group_addr: String,
    pub threshold: Threshold,
    pub max_voting_period: Duration,
}

/// This defines the different ways tallies can happen.
///
/// The total_weight used for calculating success as well as the weights of each
/// individual voter used in tallying should be snapshotted at the beginning of
/// the block at which the proposal starts (this is likely the responsibility of a
/// correct cw4 implementation).
/// See also `ThresholdResponse` in the cw3 spec.
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Threshold {
    /// Declares that a fixed weight of Yes votes is needed to pass.
    /// See `ThresholdResponse.AbsoluteCount` in the cw3 spec for details.
    AbsoluteCount { weight: u64 },

    /// Declares a percentage of the total weight that must cast Yes votes in order for
    /// a proposal to pass.
    /// See `ThresholdResponse.AbsolutePercentage` in the cw3 spec for details.
    AbsolutePercentage { percentage: Decimal },

    /// Declares a `quorum` of the total votes that must participate in the election in order
    /// for the vote to be considered at all.
    /// See `ThresholdResponse.ThresholdQuorum` in the cw3 spec for details.
    ThresholdQuorum { threshold: Decimal, quorum: Decimal },
}

impl Threshold {
    /// returns error if this is an unreachable value,
    /// given a total weight of all members in the group
    pub fn validate(&self, total_weight: u64) -> Result<(), ContractError> {
        match self {
            Threshold::AbsoluteCount {
                weight: weight_needed,
            } => {
                if *weight_needed == 0 {
                    Err(ContractError::ZeroWeight {})
                } else if *weight_needed > total_weight {
                    Err(ContractError::UnreachableWeight {})
                } else {
                    Ok(())
                }
            }
            Threshold::AbsolutePercentage {
                percentage: percentage_needed,
            } => valid_threshold(percentage_needed),
            Threshold::ThresholdQuorum {
                threshold,
                quorum: quroum,
            } => {
                valid_threshold(threshold)?;
                valid_quorum(quroum)
            }
        }
    }

    /// Creates a response from the saved data, just missing the total_weight info
    pub fn to_response(&self, total_weight: u64) -> ThresholdResponse {
        match self.clone() {
            Threshold::AbsoluteCount { weight } => ThresholdResponse::AbsoluteCount {
                weight,
                total_weight,
            },
            Threshold::AbsolutePercentage { percentage } => ThresholdResponse::AbsolutePercentage {
                percentage,
                total_weight,
            },
            Threshold::ThresholdQuorum { threshold, quorum } => {
                ThresholdResponse::ThresholdQuorum {
                    threshold,
                    quorum,
                    total_weight,
                }
            }
        }
    }
}

/// Asserts that the 0.5 < percent <= 1.0
fn valid_threshold(percent: &Decimal) -> Result<(), ContractError> {
    if *percent > Decimal::percent(100) || *percent < Decimal::percent(50) {
        Err(ContractError::InvalidThreshold {})
    } else {
        Ok(())
    }
}

/// Asserts that the 0.5 < percent <= 1.0
fn valid_quorum(percent: &Decimal) -> Result<(), ContractError> {
    if percent.is_zero() {
        Err(ContractError::ZeroQuorumThreshold {})
    } else if *percent > Decimal::one() {
        Err(ContractError::UnreachableQuorumThreshold {})
    } else {
        Ok(())
    }
}

// TODO: add some T variants? Maybe good enough as fixed Empty for now
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Propose {
        title: String,
        description: String,
        msgs: Vec<CosmosMsg<Empty>>,
        // note: we ignore API-spec'd earliest if passed, always opens immediately
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
    /// Handles update hook messages from the group contract
    MemberChangedHook(MemberChangedHookMsg),
}

// We can also add this as a cw3 extension
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Return ThresholdResponse
    Threshold {},
    /// Returns ProposalResponse
    Proposal { proposal_id: u64 },
    /// Returns ProposalListResponse
    ListProposals {
        start_after: Option<u64>,
        limit: Option<u32>,
    },
    /// Returns ProposalListResponse
    ReverseProposals {
        start_before: Option<u64>,
        limit: Option<u32>,
    },
    /// Returns VoteResponse
    Vote { proposal_id: u64, voter: String },
    /// Returns VoteListResponse
    ListVotes {
        proposal_id: u64,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// Returns VoterInfo
    Voter { address: String },
    /// Returns VoterListResponse
    ListVoters {
        start_after: Option<String>,
        limit: Option<u32>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_quorum_percentage() {
        // TODO: test the error messages

        // 0 is never a valid percentage
        let err = valid_quorum(&Decimal::zero()).unwrap_err();
        assert_eq!(
            err.to_string(),
            ContractError::ZeroQuorumThreshold {}.to_string()
        );

        // 100% is
        valid_quorum(&Decimal::one()).unwrap();

        // 101% is not
        let err = valid_quorum(&Decimal::percent(101)).unwrap_err();
        assert_eq!(
            err.to_string(),
            ContractError::UnreachableQuorumThreshold {}.to_string()
        );
        // not 100.1%
        let err = valid_quorum(&Decimal::permille(1001)).unwrap_err();
        assert_eq!(
            err.to_string(),
            ContractError::UnreachableQuorumThreshold {}.to_string()
        );
    }

    #[test]
    fn validate_threshold_percentage() {
        // other values in between 0.5 and 1 are valid
        valid_threshold(&Decimal::percent(51)).unwrap();
        valid_threshold(&Decimal::percent(67)).unwrap();
        valid_threshold(&Decimal::percent(99)).unwrap();
        let err = valid_threshold(&Decimal::percent(101)).unwrap_err();
        assert_eq!(
            err.to_string(),
            ContractError::InvalidThreshold {}.to_string()
        );
    }

    #[test]
    fn validate_threshold() {
        // absolute count ensures 0 < required <= total_weight
        let err = Threshold::AbsoluteCount { weight: 0 }
            .validate(5)
            .unwrap_err();
        // TODO: remove to_string() when PartialEq implemented
        assert_eq!(err.to_string(), ContractError::ZeroWeight {}.to_string());
        let err = Threshold::AbsoluteCount { weight: 6 }
            .validate(5)
            .unwrap_err();
        assert_eq!(
            err.to_string(),
            ContractError::UnreachableWeight {}.to_string()
        );

        Threshold::AbsoluteCount { weight: 1 }.validate(5).unwrap();
        Threshold::AbsoluteCount { weight: 5 }.validate(5).unwrap();

        // AbsolutePercentage just enforces valid_percentage (tested above)
        let err = Threshold::AbsolutePercentage {
            percentage: Decimal::zero(),
        }
        .validate(5)
        .unwrap_err();
        assert_eq!(
            err.to_string(),
            ContractError::InvalidThreshold {}.to_string()
        );
        Threshold::AbsolutePercentage {
            percentage: Decimal::percent(51),
        }
        .validate(5)
        .unwrap();

        // Quorum enforces both valid just enforces valid_percentage (tested above)
        Threshold::ThresholdQuorum {
            threshold: Decimal::percent(51),
            quorum: Decimal::percent(40),
        }
        .validate(5)
        .unwrap();
        let err = Threshold::ThresholdQuorum {
            threshold: Decimal::percent(101),
            quorum: Decimal::percent(40),
        }
        .validate(5)
        .unwrap_err();
        assert_eq!(
            err.to_string(),
            ContractError::InvalidThreshold {}.to_string()
        );
        let err = Threshold::ThresholdQuorum {
            threshold: Decimal::percent(51),
            quorum: Decimal::percent(0),
        }
        .validate(5)
        .unwrap_err();
        assert_eq!(
            err.to_string(),
            ContractError::ZeroQuorumThreshold {}.to_string()
        );
    }

    #[test]
    fn threshold_response() {
        let total_weight: u64 = 100;

        let res = Threshold::AbsoluteCount { weight: 42 }.to_response(total_weight);
        assert_eq!(
            res,
            ThresholdResponse::AbsoluteCount {
                weight: 42,
                total_weight
            }
        );

        let res = Threshold::AbsolutePercentage {
            percentage: Decimal::percent(51),
        }
        .to_response(total_weight);
        assert_eq!(
            res,
            ThresholdResponse::AbsolutePercentage {
                percentage: Decimal::percent(51),
                total_weight
            }
        );

        let res = Threshold::ThresholdQuorum {
            threshold: Decimal::percent(66),
            quorum: Decimal::percent(50),
        }
        .to_response(total_weight);
        assert_eq!(
            res,
            ThresholdResponse::ThresholdQuorum {
                threshold: Decimal::percent(66),
                quorum: Decimal::percent(50),
                total_weight
            }
        );
    }
}
