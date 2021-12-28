use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::Duration;
use cosmwasm_std::{BlockInfo, StdError, StdResult, Timestamp};
use std::cmp::Ordering;
use std::fmt;
use std::ops::Add;

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
/// Scheduled represents a point in time when an event happens.
/// It can compare with a BlockInfo and will return is_triggered() == true
/// once the condition is hit (and for every block in the future)
pub enum Scheduled {
    /// AtHeight will schedule when `env.block.height` >= height
    AtHeight(u64),
    /// AtTime will schedule when `env.block.time` >= time
    AtTime(Timestamp),
}

impl fmt::Display for Scheduled {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Scheduled::AtHeight(height) => write!(f, "scheduled height: {}", height),
            Scheduled::AtTime(time) => write!(f, "scheduled time: {}", time),
        }
    }
}

impl Scheduled {
    #[allow(dead_code)]
    pub fn is_triggered(&self, block: &BlockInfo) -> bool {
        match self {
            Scheduled::AtHeight(height) => block.height >= *height,
            Scheduled::AtTime(time) => block.time >= *time,
        }
    }
}

impl Add<Duration> for Scheduled {
    type Output = StdResult<Scheduled>;

    fn add(self, duration: Duration) -> StdResult<Scheduled> {
        match (self, duration) {
            (Scheduled::AtTime(t), Duration::Time(delta)) => {
                Ok(Scheduled::AtTime(t.plus_seconds(delta)))
            }
            (Scheduled::AtHeight(h), Duration::Height(delta)) => Ok(Scheduled::AtHeight(h + delta)),
            _ => Err(StdError::generic_err("Cannot add height and time")),
        }
    }
}

impl PartialOrd for Scheduled {
    fn partial_cmp(&self, other: &Scheduled) -> Option<Ordering> {
        match (self, other) {
            // compare if both height or both time
            (Scheduled::AtHeight(h1), Scheduled::AtHeight(h2)) => Some(h1.cmp(h2)),
            (Scheduled::AtTime(t1), Scheduled::AtTime(t2)) => Some(t1.cmp(t2)),
            _ => None,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn compare_schedules() {
        // matching pairs
        assert!(Scheduled::AtHeight(5) < Scheduled::AtHeight(10));
        assert!(Scheduled::AtHeight(8) > Scheduled::AtHeight(7));
        assert!(
            Scheduled::AtTime(Timestamp::from_seconds(555))
                < Scheduled::AtTime(Timestamp::from_seconds(777))
        );
        assert!(
            Scheduled::AtTime(Timestamp::from_seconds(86))
                < Scheduled::AtTime(Timestamp::from_seconds(100))
        );

        // what happens for the uncomparables?? all compares are false
        assert_eq!(
            None,
            Scheduled::AtTime(Timestamp::from_seconds(1000)).partial_cmp(&Scheduled::AtHeight(230))
        );
        assert_eq!(
            Scheduled::AtTime(Timestamp::from_seconds(1000)).partial_cmp(&Scheduled::AtHeight(230)),
            None
        );
        assert_eq!(
            Scheduled::AtTime(Timestamp::from_seconds(1000)).partial_cmp(&Scheduled::AtHeight(230)),
            None
        );
        assert!(!(Scheduled::AtTime(Timestamp::from_seconds(1000)) == Scheduled::AtHeight(230)));
    }

    #[test]
    fn schedule_addition() {
        // height
        let end = Scheduled::AtHeight(12345) + Duration::Height(400);
        assert_eq!(end.unwrap(), Scheduled::AtHeight(12745));

        // time
        let end = Scheduled::AtTime(Timestamp::from_seconds(55544433)) + Duration::Time(40300);
        assert_eq!(
            end.unwrap(),
            Scheduled::AtTime(Timestamp::from_seconds(55584733))
        );

        // mismatched
        let end = Scheduled::AtHeight(12345) + Duration::Time(1500);
        end.unwrap_err();
    }
}
