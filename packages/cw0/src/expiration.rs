use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{BlockInfo, StdError, StdResult, Timestamp};
use std::cmp::Ordering;
use std::fmt;
use std::ops::{Add, Mul};

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
/// Expiration represents a point in time when some event happens.
/// It can compare with a BlockInfo and will return is_expired() == true
/// once the condition is hit (and for every block in the future)
pub enum Expiration {
    /// AtHeight will expire when `env.block.height` >= height
    AtHeight(u64),
    /// AtTime will expire when `env.block.time` >= time
    AtTime(Timestamp),
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
            (Expiration::AtTime(t), Duration::Time(delta)) => {
                Ok(Expiration::AtTime(t.plus_seconds(delta)))
            }
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

pub const HOUR: Duration = Duration::Time(60 * 60);
pub const DAY: Duration = Duration::Time(24 * 60 * 60);
pub const WEEK: Duration = Duration::Time(7 * 24 * 60 * 60);

/// Duration is a delta of time. You can add it to a BlockInfo or Expiration to
/// move that further in the future. Note that an height-based Duration and
/// a time-based Expiration cannot be combined
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Duration {
    Height(u64),
    /// Time in seconds
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
            Duration::Time(t) => Expiration::AtTime(block.time.plus_seconds(*t)),
        }
    }

    // creates a number just a little bigger, so we can use it to pass expiration point
    pub fn plus_one(&self) -> Duration {
        match self {
            Duration::Height(h) => Duration::Height(h + 1),
            Duration::Time(t) => Duration::Time(t + 1),
        }
    }
}

impl Add<Duration> for Duration {
    type Output = StdResult<Duration>;

    fn add(self, rhs: Duration) -> StdResult<Duration> {
        match (self, rhs) {
            (Duration::Time(t), Duration::Time(t2)) => Ok(Duration::Time(t + t2)),
            (Duration::Height(h), Duration::Height(h2)) => Ok(Duration::Height(h + h2)),
            _ => Err(StdError::generic_err("Cannot add height and time")),
        }
    }
}

impl Mul<u64> for Duration {
    type Output = Duration;

    fn mul(self, rhs: u64) -> Self::Output {
        match self {
            Duration::Time(t) => Duration::Time(t * rhs),
            Duration::Height(h) => Duration::Height(h * rhs),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn compare_expiration() {
        // matching pairs
        assert!(Expiration::AtHeight(5) < Expiration::AtHeight(10));
        assert!(Expiration::AtHeight(8) > Expiration::AtHeight(7));
        assert!(
            Expiration::AtTime(Timestamp::from_seconds(555))
                < Expiration::AtTime(Timestamp::from_seconds(777))
        );
        assert!(
            Expiration::AtTime(Timestamp::from_seconds(86))
                < Expiration::AtTime(Timestamp::from_seconds(100))
        );

        // never as infinity
        assert!(Expiration::AtHeight(500000) < Expiration::Never {});
        assert!(Expiration::Never {} > Expiration::AtTime(Timestamp::from_seconds(500000)));

        // what happens for the uncomparables?? all compares are false
        assert_eq!(
            None,
            Expiration::AtTime(Timestamp::from_seconds(1000))
                .partial_cmp(&Expiration::AtHeight(230))
        );
        assert_eq!(
            Expiration::AtTime(Timestamp::from_seconds(1000))
                .partial_cmp(&Expiration::AtHeight(230)),
            None
        );
        assert_eq!(
            Expiration::AtTime(Timestamp::from_seconds(1000))
                .partial_cmp(&Expiration::AtHeight(230)),
            None
        );
        assert!(!(Expiration::AtTime(Timestamp::from_seconds(1000)) == Expiration::AtHeight(230)));
    }

    #[test]
    fn expiration_addition() {
        // height
        let end = Expiration::AtHeight(12345) + Duration::Height(400);
        assert_eq!(end.unwrap(), Expiration::AtHeight(12745));

        // time
        let end = Expiration::AtTime(Timestamp::from_seconds(55544433)) + Duration::Time(40300);
        assert_eq!(
            end.unwrap(),
            Expiration::AtTime(Timestamp::from_seconds(55584733))
        );

        // never
        let end = Expiration::Never {} + Duration::Time(40300);
        assert_eq!(end.unwrap(), Expiration::Never {});

        // mismatched
        let end = Expiration::AtHeight(12345) + Duration::Time(1500);
        end.unwrap_err();

        // // not possible other way
        // let end = Duration::Time(1000) + Expiration::AtTime(50000);
        // assert_eq!(end.unwrap(), Expiration::AtTime(51000));
    }

    #[test]
    fn block_plus_duration() {
        let block = BlockInfo {
            height: 1000,
            time: Timestamp::from_seconds(7777),
            chain_id: "foo".to_string(),
        };

        let end = Duration::Height(456).after(&block);
        assert_eq!(Expiration::AtHeight(1456), end);

        let end = Duration::Time(1212).after(&block);
        assert_eq!(Expiration::AtTime(Timestamp::from_seconds(8989)), end);
    }

    #[test]
    fn duration_math() {
        let long = (Duration::Height(444) + Duration::Height(555)).unwrap();
        assert_eq!(Duration::Height(999), long);

        let days = DAY * 3;
        assert_eq!(Duration::Time(3 * 24 * 60 * 60), days);
    }
}
