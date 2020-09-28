use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{BlockInfo, StdError, StdResult};
use std::cmp::Ordering;
use std::fmt;
use std::ops::Add;

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
/// Expiration represents a point in time when some event happens.
/// It can compare with a BlockInfo and will return is_expired() == true
/// once the condition is hit (and for every block in the future)
pub enum Expiration {
    /// AtHeight will expire when `env.block.height` >= height
    AtHeight(u64),
    /// AtTime will expire when `env.block.time` >= time
    AtTime(u64),
    /// Never will never expire. Used to express the empty variant
    Never {},
}

impl fmt::Display for Expiration {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Expiration::AtHeight(height) => write!(f, "expiration height: {}", height),
            Expiration::AtTime(time) => write!(f, "expiration time: {}", time),
            Expiration::Never {} => write!(f, "expiration: never"),
        }
    }
}

/// The default (empty value) is to never expire
impl Default for Expiration {
    fn default() -> Self {
        Expiration::Never {}
    }
}

impl Expiration {
    pub fn is_expired(&self, block: &BlockInfo) -> bool {
        match self {
            Expiration::AtHeight(height) => block.height >= *height,
            Expiration::AtTime(time) => block.time >= *time,
            Expiration::Never {} => false,
        }
    }
}

impl Add<Duration> for Expiration {
    type Output = StdResult<Expiration>;

    fn add(self, duration: Duration) -> StdResult<Expiration> {
        match (self, duration) {
            (Expiration::AtTime(t), Duration::Time(delta)) => Ok(Expiration::AtTime(t + delta)),
            (Expiration::AtHeight(h), Duration::Height(delta)) => {
                Ok(Expiration::AtHeight(h + delta))
            }
            (Expiration::Never {}, _) => Ok(Expiration::Never {}),
            _ => Err(StdError::generic_err("Cannot add height and time")),
        }
    }
}

// TODO: does this make sense? do we get expected info/error when None is returned???
impl PartialOrd for Expiration {
    fn partial_cmp(&self, other: &Expiration) -> Option<Ordering> {
        match (self, other) {
            // compare if both height or both time
            (Expiration::AtHeight(h1), Expiration::AtHeight(h2)) => Some(h1.cmp(h2)),
            (Expiration::AtTime(t1), Expiration::AtTime(t2)) => Some(t1.cmp(t2)),
            // if at least one is never, we can compare with anything
            (Expiration::Never {}, Expiration::Never {}) => Some(Ordering::Equal),
            (Expiration::Never {}, _) => Some(Ordering::Greater),
            (_, Expiration::Never {}) => Some(Ordering::Less),
            // if they are mis-matched finite ends, no compare possible
            _ => None,
        }
    }
}

/// Duration is a delta of time. You can add it to a BlockInfo or Expiration to
/// move that further in the future. Note that an height-based Duration and
/// a time-based Expiration cannot be combined
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Duration {
    Height(u64),
    Time(u64),
}

impl fmt::Display for Duration {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Duration::Height(height) => write!(f, "height: {}", height),
            Duration::Time(time) => write!(f, "time: {}", time),
        }
    }
}

impl Duration {
    /// Create an expiration for Duration after current block
    pub fn after(&self, block: &BlockInfo) -> Expiration {
        match self {
            Duration::Height(h) => Expiration::AtHeight(block.height + h),
            Duration::Time(t) => Expiration::AtTime(block.time + t),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    // TODO: add tests for the logic
    #[test]
    fn compare_expiration() {
        // matching pairs
        assert_eq!(true, Expiration::AtHeight(5) < Expiration::AtHeight(10));
        assert_eq!(false, Expiration::AtHeight(8) < Expiration::AtHeight(7));
        assert_eq!(true, Expiration::AtTime(555) < Expiration::AtTime(777));
        assert_eq!(false, Expiration::AtTime(86) > Expiration::AtTime(100));

        // never as infinity
        assert!(Expiration::AtHeight(500000) < Expiration::Never {});
        assert!(Expiration::Never {} > Expiration::AtTime(500000));

        // what happens for the uncomparables?? all compares are false
        assert_eq!(
            None,
            Expiration::AtTime(1000).partial_cmp(&Expiration::AtHeight(230))
        );
        assert_eq!(false, Expiration::AtTime(1000) < Expiration::AtHeight(230));
        assert_eq!(false, Expiration::AtTime(1000) > Expiration::AtHeight(230));
        assert_eq!(false, Expiration::AtTime(1000) == Expiration::AtHeight(230));
    }
}
