# Tracing contract

This contract is designed specifically to visualize the flow of messages and transactions
in the actor model.

The idea is, that it keeps the information about the whole history of calls perfomed on it,
and the history of this log itself.

## Messages

Two basic messages on this contract are:

* `{ 'touch': {} }` - creates new log entry and resolves
* `{ 'fail': {} }` - creates new log entry and immediately fails

Note, that the `fail` message would never update the log, as it would
be reverted immediately, but it never hurts to visualize it.

Third message is a bit more complicated, and it is structured as follows:

```json
{
    'forward': {
        'addr': '...',
        'msg': { ... },
        'marker': 0,
        'catch_success': true,
        'catch_failure': true,
        'fail_reply': false,
    }
}
```

This messages is logging the execution, but it also stores itself in the state separately.
Then, it prepares a subcall of messagge `msg` to the address `addr`. The `marker` field is
an `id` used for the reply so it is possible to identify the call and the reply in the log
easily. `catch_success` and `catch_failure` are two fields which determines if the `reply`
should be called on success or an error (possibly both). The last field is the additional
flag allowing to fail the transaction in the reply (it would succeed otherwise).

On the reply handling, the message send would be restored from state, and then it would
be logged again, but flagged as a reply.

Note, that `forward` message can be stacked deeply to simulate complex flows.

There are to last helper messages just for making it easier to clearly reuse contracts instead
of uploading it all over:

* `{ 'clear': {} }` - pushes the new clear log state, but doesn't clear whole log history (clears
  the "last instance"
* `{ 'reset': {} } - resets logs history

## Queries

There is only one query handled by the contract: `{ 'log': { 'depth': 1 } }`.

The message gives back whole logs history stored in the contract. There is an
optional `depth` field which limits how many history entries would be returned
(if field is omitted, whole history is returned).

The format of the single log entry is:

```json
{
    'sender': '...',
    'msg': { ... },
    `reply': false,
    'marker': 0,
}
```

The `sender` is the sender of the message (for `reply` entries it would be the contract address itself).
Then the `msg` is message handled. The `reply` is `true` if this is a `reply` handling, `false` otherwise.
The `marker` is only there for `forward` messages and `reply` to them, and it is the `id` used for the
reply (it should always be equal to the `marker` field of `forward` message).
