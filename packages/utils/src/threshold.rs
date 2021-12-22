use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Decimal, StdError};
use thiserror::Error;

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
    pub fn validate(&self, total_weight: u64) -> Result<(), ThresholdError> {
        match self {
            Threshold::AbsoluteCount {
                weight: weight_needed,
            } => {
                if *weight_needed == 0 {
                    Err(ThresholdError::ZeroWeight {})
                } else if *weight_needed > total_weight {
                    Err(ThresholdError::UnreachableWeight {})
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
fn valid_threshold(percent: &Decimal) -> Result<(), ThresholdError> {
    if *percent > Decimal::percent(100) || *percent < Decimal::percent(50) {
        Err(ThresholdError::InvalidThreshold {})
    } else {
        Ok(())
    }
}

/// Asserts that the 0.5 < percent <= 1.0
fn valid_quorum(percent: &Decimal) -> Result<(), ThresholdError> {
    if percent.is_zero() {
        Err(ThresholdError::ZeroQuorumThreshold {})
    } else if *percent > Decimal::one() {
        Err(ThresholdError::UnreachableQuorumThreshold {})
    } else {
        Ok(())
    }
}

/// This defines the different ways tallies can happen.
/// Every contract should support a subset of these, ideally all.
///
/// The total_weight used for calculating success as well as the weights of each
/// individual voter used in tallying should be snapshotted at the beginning of
/// the block at which the proposal starts (this is likely the responsibility of a
/// correct cw4 implementation).
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ThresholdResponse {
    /// Declares that a fixed weight of yes votes is needed to pass.
    /// It does not matter how many no votes are cast, or how many do not vote,
    /// as long as `weight` yes votes are cast.
    ///
    /// This is the simplest format and usually suitable for small multisigs of trusted parties,
    /// like 3 of 5. (weight: 3, total_weight: 5)
    ///
    /// A proposal of this type can pass early as soon as the needed weight of yes votes has been cast.
    AbsoluteCount { weight: u64, total_weight: u64 },

    /// Declares a percentage of the total weight that must cast Yes votes, in order for
    /// a proposal to pass. The passing weight is computed over the total weight minus the weight of the
    /// abstained votes.
    ///
    /// This is useful for similar circumstances as `AbsoluteCount`, where we have a relatively
    /// small set of voters, and participation is required.
    /// It is understood that if the voting set (group) changes between different proposals that
    /// refer to the same group, each proposal will work with a different set of voter weights
    /// (the ones snapshotted at proposal creation), and the passing weight for each proposal
    /// will be computed based on the absolute percentage, times the total weights of the members
    /// at the time of each proposal creation.
    ///
    /// Example: we set `percentage` to 51%. Proposal 1 starts when there is a `total_weight` of 5.
    /// This will require 3 weight of Yes votes in order to pass. Later, the Proposal 2 starts but the
    /// `total_weight` of the group has increased to 9. That proposal will then automatically
    /// require 5 Yes of 9 to pass, rather than 3 yes of 9 as would be the case with `AbsoluteCount`.
    AbsolutePercentage {
        percentage: Decimal,
        total_weight: u64,
    },

    /// In addition to a `threshold`, declares a `quorum` of the total votes that must participate
    /// in the election in order for the vote to be considered at all. Within the votes that
    /// were cast, it requires `threshold` votes in favor. That is calculated by ignoring
    /// the Abstain votes (they count towards `quorum`, but do not influence `threshold`).
    /// That is, we calculate `Yes / (Yes + No + Veto)` and compare it with `threshold` to consider
    /// if the proposal was passed.
    ///
    /// It is rather difficult for a proposal of this type to pass early. That can only happen if
    /// the required quorum has been already met, and there are already enough Yes votes for the
    /// proposal to pass.
    ///
    /// 30% Yes votes, 10% No votes, and 20% Abstain would pass early if quorum <= 60%
    /// (who has cast votes) and if the threshold is <= 37.5% (the remaining 40% voting
    /// no => 30% yes + 50% no). Once the voting period has passed with no additional votes,
    /// that same proposal would be considered successful if quorum <= 60% and threshold <= 75%
    /// (percent in favor if we ignore abstain votes).
    ///
    /// This type is more common in general elections, where participation is often expected to
    /// be low, and `AbsolutePercentage` would either be too high to pass anything,
    /// or allow low percentages to pass, independently of if there was high participation in the
    /// election or not.
    ThresholdQuorum {
        threshold: Decimal,
        quorum: Decimal,
        total_weight: u64,
    },
}

#[derive(Error, Debug, PartialEq)]
pub enum ThresholdError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Invalid voting threshold percentage, must be in the 0.5-1.0 range")]
    InvalidThreshold {},

    #[error("Required quorum threshold cannot be zero")]
    ZeroQuorumThreshold {},

    #[error("Not possible to reach required quorum threshold")]
    UnreachableQuorumThreshold {},

    #[error("Required weight cannot be zero")]
    ZeroWeight {},

    #[error("Not possible to reach required (passing) weight")]
    UnreachableWeight {},
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
            ThresholdError::ZeroQuorumThreshold {}.to_string()
        );

        // 100% is
        valid_quorum(&Decimal::one()).unwrap();

        // 101% is not
        let err = valid_quorum(&Decimal::percent(101)).unwrap_err();
        assert_eq!(
            err.to_string(),
            ThresholdError::UnreachableQuorumThreshold {}.to_string()
        );
        // not 100.1%
        let err = valid_quorum(&Decimal::permille(1001)).unwrap_err();
        assert_eq!(
            err.to_string(),
            ThresholdError::UnreachableQuorumThreshold {}.to_string()
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
            ThresholdError::InvalidThreshold {}.to_string()
        );
    }

    #[test]
    fn validate_threshold() {
        // absolute count ensures 0 < required <= total_weight
        let err = Threshold::AbsoluteCount { weight: 0 }
            .validate(5)
            .unwrap_err();
        // TODO: remove to_string() when PartialEq implemented
        assert_eq!(err.to_string(), ThresholdError::ZeroWeight {}.to_string());
        let err = Threshold::AbsoluteCount { weight: 6 }
            .validate(5)
            .unwrap_err();
        assert_eq!(
            err.to_string(),
            ThresholdError::UnreachableWeight {}.to_string()
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
            ThresholdError::InvalidThreshold {}.to_string()
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
            ThresholdError::InvalidThreshold {}.to_string()
        );
        let err = Threshold::ThresholdQuorum {
            threshold: Decimal::percent(51),
            quorum: Decimal::percent(0),
        }
        .validate(5)
        .unwrap_err();
        assert_eq!(
            err.to_string(),
            ThresholdError::ZeroQuorumThreshold {}.to_string()
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
