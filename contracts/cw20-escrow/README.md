# CW20 Escrow

This is an escrow meta-contract that allows multiple users to
create independent escrows. Each escrow has a sender, recipient,
and arbiter. It also has a unique id (for future calls to reference it)
and an optional timeout.

The basic function is the sender creates an escrow with funds.
The arbiter may at any time decide to release the funds to either
the intended recipient or the original sender (but no one else),
and if it passes with optional timeout, anyone can refund the locked
tokens to the original sender.

We also add a function called "top_up", which allows anyone to add more
funds to the contract at any time.

## Token types

This contract is meant not just to be functional, but also to work as a simple
example of an CW20 "Receiver". And demonstrate how the same calls can be fed
native tokens (via typical `ExecuteMsg` route), or cw20 tokens (via `Receiver` interface).

Both `create` and `top_up` can be called directly (with a payload of native tokens),
or from a cw20 contract using the [Receiver Interface](../../packages/cw20/README.md#receiver).
This means we can load the escrow with any number of native or cw20 tokens (or a mix),
allow of which get released when the arbiter decides.

## Running this contract

You will need Rust 1.44.1+ with `wasm32-unknown-unknown` target installed.

You can run unit tests on this via: 

`cargo test`

Once you are happy with the content, you can compile it to wasm via:

```
RUSTFLAGS='-C link-arg=-s' cargo wasm
cp ../../target/wasm32-unknown-unknown/release/cw20_escrow.wasm .
ls -l cw20_escrow.wasm
sha256sum cw20_escrow.wasm
```

Or for a production-ready (optimized) build, run a build command in the
the repository root: https://github.com/CosmWasm/cw-plus#compiling.
