use assert_matches::assert_matches;

use crate::{msg::ExecuteMsg, state::LogEntry};

use self::suite::Suite;

mod suite;

#[test]
fn touch() {
    let client = "client";
    let mut suite = Suite::new();

    suite.touch(client).unwrap();
    let log = suite.log(None).unwrap();

    assert_matches!(&log.last().unwrap()[..], &[LogEntry {
        ref msg,
        ..
    }] if *msg == ExecuteMsg::Touch {  });
}

#[test]
fn fail() {
    let client = "client";
    let mut suite = Suite::new();

    suite.fail(client).unwrap_err();
    let log = suite.log(None).unwrap();

    assert_matches!(&log.last().unwrap()[..], &[]);
}

#[test]
fn clear() {
    let client = "client";
    let mut suite = Suite::new();

    suite.touch(client).unwrap();
    suite.clear(client).unwrap();
    let log = suite.log(None).unwrap();

    // 3 as it contains initial empty log, then the log after `touch`, and then
    // the log after clear.
    assert_matches!(&log.last().unwrap()[..], &[]);
}

#[test]
fn reset() {
    let client = "client";
    let mut suite = Suite::new();

    suite.touch(client).unwrap();
    suite.reset(client).unwrap();
    let log = suite.log(None).unwrap();

    assert_eq!(1, log.len());
}
