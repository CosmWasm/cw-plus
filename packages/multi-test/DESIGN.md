# Multitest Design

Multitest is a design to simulate a blockchain environment in pure Rust.
This allows us to run unit tests that involve contract -> contract,
and contract -> bank interactions. This is not intended to be a full blockchain app
but to simulate the Cosmos SDK x/wasm module close enough to gain confidence in
multi-contract deployements before testing them on a live blockchain.

This explains some of the design for those who want to use the API, as well as those
who want to look under the hood.

## Key APIs

### App

The main entry point to the system is called `App`, which represents a blockchain app.
It maintains an idea of block height and time, which you can update to simulate multiple
blocks. You can use `app.update_block(next_block)` to increment timestamp by 5s and height by 1
(simulating a new block) or you can write any other mutator to advance more.

It exposes an entry point `App.execute` that allows us to execute any `CosmosMsg`
and it wraps it as an atomic transaction. That is, only if `execute` returns success, will the state
be committed. It returns the data and a list of Events on successful execution or an `Err(String)`
on error. There are some helper methods tied to the `Executor` trait that create the `CosmosMsg` for
you to provide a less verbose API. `instantiate_contract`,`execute_contract`, and `send_tokens` are exposed
for your convenience in writing tests. Each execute one `CosmosMsg` atomically as if it was submitted by a user.
(You can also use `execute_multi` if you wish to run multiple message together that revert all state if any fail).

The other key entry point to `App` is the `Querier` interface that it implements. In particular, you
can use `App.wrap()` to get a `QuerierWrapper`, which provides all kinds of nice APIs to query the
blockchain, like `all_balances` and `query_wasm_smart`. Putting this together, you have one `Storage` wrapped
into an application, where you can execute contracts and bank, query them easily, and update the current
`BlockInfo`, in an API that is not very verbose or cumbersome. Under the hood it will process all messages
returned from contracts, move "bank" tokens and call into other contracts.

Note: This properly handles submessages and reply blocks.

Note: While the API currently supports custom messages, we don't currently have a way to handle/process them.

### Contracts

Before you can call contracts, you must `instantiate` them. And to instantiate them, you need a `code_id`.
In `wasmd`, this `code_id` points to some stored Wasm code that is then run. In multitest, we use it to
point to a `Box<dyn Contract>` that should be run. That is, you need to implement the `Contract` trait
and then add to to the app via `app.store_code(my_contract)`.

The `Contract` trait defines the major entry points to any CosmWasm contract: `execute`, `instantiate`, `query`,
`sudo`, and `reply` (for submessages). Migration and IBC are currently not supported.

In order to easily implement `Contract` from some existing contract code, we use the `ContractWrapper` struct,
which takes some function pointers and combines them. You can look in `test_helpers.rs` for some examples
or how to do so (and useful mocks for some test cases). Here is an example of wrapping a CosmWasm contract into
a `Contract` trait to add to an `App`:

```rust
pub fn contract_reflect() -> Box<dyn Contract<CustomMsg>> {
    let contract = ContractWrapper::new(execute_reflect, instantiate_reflect, query_reflect)
        .with_reply(reply_reflect);
    Box::new(contract)
}
```

If you are not using custom messages in your contract, you can just use `dyn Contract<Empty>`.

### Examples

The best intro is most likely `integration.rs` in `cw20-escrow`, which shows sending and releasing native tokens in
an escrow, as well as sending and releasing cw20 tokens. The first one updates the global bank ledger, the second
actually shows how we can test orchestrating multiple contracts.

## Implementation

### StorageTransaction

### Modules

### Router
