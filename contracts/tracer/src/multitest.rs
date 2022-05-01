use assert_matches::assert_matches;
use cosmwasm_std::to_binary;

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
    assert_matches!(&log.last().unwrap()[..], &[]);
}

#[test]
fn forward_success() {
    let client = "client";
    let mut suite = Suite::new();

    suite
        .forward(
            client,
            &suite.contract_addr(),
            ExecuteMsg::Touch {},
            1,
            false,
            false,
            false,
        )
        .unwrap();

    let log = suite.log(None).unwrap();
    assert_matches!(
        &log.last().unwrap()[..],
        &[
            LogEntry {
                msg: ExecuteMsg::Forward { .. },
                reply: false,
                marker: Some(1),
                ..
            },
            LogEntry {
                msg: ExecuteMsg::Touch { .. },
                reply: false,
                ..
            }
        ]
    );
}

#[test]
fn forward_fail_rollbacks_whole() {
    let client = "client";
    let mut suite = Suite::new();

    suite
        .forward(
            client,
            &suite.contract_addr(),
            ExecuteMsg::Fail {},
            1,
            false,
            false,
            false,
        )
        .unwrap_err();

    let log = suite.log(None).unwrap();
    assert_matches!(&log.last().unwrap()[..], &[]);
}

#[test]
fn reply_preserves_marker() {
    let client = "client";
    let mut suite = Suite::new();

    suite
        .forward(
            client,
            &suite.contract_addr(),
            ExecuteMsg::Touch {},
            1,
            true,
            false,
            false,
        )
        .unwrap();

    let log = suite.log(None).unwrap();
    assert_matches!(
        &log.last().unwrap()[..],
        &[LogEntry {
            msg: ref msg1 @ ExecuteMsg::Forward { marker: 1, .. },
            reply: false,
            marker: Some(1),
            ..
        }, LogEntry {
            msg: ExecuteMsg::Touch { .. },
            reply: false,
            ..
        }, LogEntry {
            msg: ref msg2 @ ExecuteMsg::Forward { marker: 1, .. },
            reply: true,
            marker: Some(1),
            ..
        }] if msg1 == msg2
    );
}

#[test]
fn fail_in_reply_fails_whole() {
    let client = "client";
    let mut suite = Suite::new();

    suite
        .forward(
            client,
            &suite.contract_addr(),
            ExecuteMsg::Touch {},
            1,
            true,
            false,
            true,
        )
        .unwrap_err();

    let log = suite.log(None).unwrap();
    assert_matches!(&log.last().unwrap()[..], &[]);
}

#[test]
fn recover_in_reply_after_fail() {
    let client = "client";
    let mut suite = Suite::new();

    suite
        .forward(
            client,
            &suite.contract_addr(),
            ExecuteMsg::Fail {},
            1,
            false,
            true,
            false,
        )
        .unwrap();

    let log = suite.log(None).unwrap();
    assert_matches!(
        &log.last().unwrap()[..],
        &[LogEntry {
            msg: ref msg1 @ ExecuteMsg::Forward { marker: 1, .. },
            reply: false,
            marker: Some(1),
            ..
        }, LogEntry {
            msg: ref msg2 @ ExecuteMsg::Forward { marker: 1, .. },
            reply: true,
            marker: Some(1),
            ..
        }] if msg1 == msg2
    );
}

#[test]
fn reentrancy_double_forward() {
    // Test showing reentrancy problem with recursive calls to
    // contracts. In the nutshell: the message for `forward`
    // message is stored in internal state, so it can be retrieved
    // for the reply handling. Unfortunatelly, when the `forward`
    // calls another `forward`, the message is overwritten and
    // never restored. This causes potential issue, that if there
    // is a possibility of such recursive calls, it cannot be
    // thrusted, that any kind of intermediate data stored for reply
    // handling would be consistent.
    //
    // However what behaves correctly is `id` of the reply is porperly
    // preserved. Therefore to avoid this kind of issues, id for submessages
    // should be assigned dynamically (so for each recursive call there is
    // different id), and any metadata to be preserved between call and
    // reply should be stored per id (eg. in map - in can be cleaned up
    // in reply handling).
    //
    // Additionally for this very contract this test proves, that the `msg`
    // field should not be used to verify the replies - if the `reply` is
    // `true` in log, the `marker` field should be used to determine which
    // message reply is called.
    let client = "client";
    let mut suite = Suite::new();

    let touch = ExecuteMsg::Touch {};
    let touch = to_binary(&touch).unwrap();

    // Messages structure:
    // {
    //   "forward": { // later referred as "first message"
    //     addr: "..",
    //     marker: 1,
    //     msg: {
    //       "forward": { // "second message"
    //         addr: "..",
    //         marker: 2,
    //         msg: {
    //           "touch": {} // "third messsage"
    //         },
    //         catch_success: true,
    //       }
    //     },
    //     catch_success: true,
    //   }
    // }
    let forward_msg = ExecuteMsg::Forward {
        addr: suite.contract_addr(),
        msg: touch,
        marker: 2,
        catch_success: true,
        catch_failure: false,
        fail_reply: false,
    };
    suite
        .forward(
            client,
            &suite.contract_addr(),
            forward_msg,
            1,
            true,
            false,
            false,
        )
        .unwrap();

    let log = suite.log(None).unwrap();
    assert_matches!(
        &log.last().unwrap()[..],
        &[LogEntry {
            // First message handled
            msg: ref msg1 @ ExecuteMsg::Forward { marker: 1, .. },
            reply: false,
            // Marker id proper for first message
            marker: Some(1),
            ..
        }, LogEntry {
            // Second message handled
            msg: ref msg2 @ ExecuteMsg::Forward { marker: 2, .. },
            reply: false,
            // Marker id proper for second message
            marker: Some(2),
            ..
        }, LogEntry {
            // Third message - just touch
            msg: ExecuteMsg::Touch {},
            ..
        }, LogEntry {
            // Second message reply - proper message stored
            msg: ref reply2 @ ExecuteMsg::Forward { marker: 2, .. },
            reply: true,
            // Marker id proper for second message
            marker: Some(2),
            ..
        }, LogEntry {
            // Second message overwritten by recursive call
            msg: ref reply1 @ ExecuteMsg::Forward { marker: 2, .. },
            reply: true,
            // Marker id proper for first message
            marker: Some(1),
            ..
        }] if msg1 != reply1 && msg2 == reply2 && reply1 == reply2
    );
}

#[test]
fn reentrancy_without_second_reply() {
    // Same as `reentrancy_double_forward` but without reply for the
    // second message to proof that the overwritting happens in message,
    // not in reply.

    let client = "client";
    let mut suite = Suite::new();

    let touch = ExecuteMsg::Touch {};
    let touch = to_binary(&touch).unwrap();

    // Messages structure:
    // {
    //   "forward": { // later referred as "first message"
    //     addr: "..",
    //     marker: 1,
    //     msg: {
    //       "forward": { // "second message"
    //         addr: "..",
    //         marker: 2,
    //         msg: {
    //           "touch": {} // "third messsage"
    //         },
    //       }
    //     },
    //     catch_success: true,
    //   }
    // }
    let forward_msg = ExecuteMsg::Forward {
        addr: suite.contract_addr(),
        msg: touch,
        marker: 2,
        catch_success: false,
        catch_failure: false,
        fail_reply: false,
    };
    suite
        .forward(
            client,
            &suite.contract_addr(),
            forward_msg,
            1,
            true,
            false,
            false,
        )
        .unwrap();

    let log = suite.log(None).unwrap();
    assert_matches!(
        &log.last().unwrap()[..],
        &[LogEntry {
            // First message handled
            msg: ref msg1 @ ExecuteMsg::Forward { marker: 1, .. },
            reply: false,
            // Marker id proper for first message
            marker: Some(1),
            ..
        }, LogEntry {
            // Second message handled
            msg: ref msg2 @ ExecuteMsg::Forward { marker: 2, .. },
            reply: false,
            // Marker id proper for second message
            marker: Some(2),
            ..
        }, LogEntry {
            // Third message - just touch
            msg: ExecuteMsg::Touch {},
            ..
        }, LogEntry {
            // Second message overwritten by recursive call
            msg: ref reply1 @ ExecuteMsg::Forward { marker: 2, .. },
            reply: true,
            // Marker id proper for first message
            marker: Some(1),
            ..
        }] if msg1 != reply1 && msg2 == reply1
    );
}

#[test]
fn fail_propagates_till_first_catch() {
    let client = "client";
    let mut suite = Suite::new();

    let fail = ExecuteMsg::Fail {};
    let fail = to_binary(&fail).unwrap();

    // Messages structure:
    // {
    //   "forward": { // later referred as "first message"
    //     addr: "..",
    //     marker: 1,
    //     msg: {
    //       "forward": { // "second message"
    //         addr: "..",
    //         marker: 2,
    //         msg: {
    //           "fail": {} // "third messsage"
    //         },
    //         catch_failure: true,
    //       }
    //     },
    //   }
    // }
    let forward_msg = ExecuteMsg::Forward {
        addr: suite.contract_addr(),
        msg: fail,
        marker: 2,
        catch_success: false,
        catch_failure: true,
        fail_reply: false,
    };
    suite
        .forward(
            client,
            &suite.contract_addr(),
            forward_msg,
            1,
            false,
            false,
            false,
        )
        .unwrap();

    let log = suite.log(None).unwrap();
    assert_matches!(
        &log.last().unwrap()[..],
        &[
            LogEntry {
                msg: ExecuteMsg::Forward { .. },
                reply: false,
                marker: Some(1),
                ..
            },
            LogEntry {
                msg: ExecuteMsg::Forward { .. },
                reply: false,
                marker: Some(2),
                ..
            },
            LogEntry {
                reply: true,
                marker: Some(2),
                ..
            }
        ]
    );
}
