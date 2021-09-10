# CW1 Whitelist

This may be the simplest implementation of CW1, a whitelist of addresses.
It contains a set of admins that are defined upon creation.
Any of those admins may `Execute` any message via the contract,
per the CW1 spec.

To make this slighly less minimalistic, you can allow the admin set
to be mutable or immutable. If it is mutable, then any admin may
(a) change the admin set and (b) freeze it (making it immutable).

While largely an example contract for CW1, this has various real-world use-cases,
such as a common account that is shared among multiple trusted devices,
or trading an entire account (used as 1 of 1 mutable). Most of the time,
this can be used as a framework to build your own, 
more advanced cw1 implementations.

## Allowing Custom Messages

By default, this doesn't support `CustomMsg` in order to be fully generic
among blockchains. However, all types are Generic over `T`, and this is only
fixed in `handle`. You can import this contract and just redefine your `handle`
function, setting a different parameter to `ExecuteMsg`, and you can produce
a chain-specific message.

## Running this contract

You will need Rust 1.44.1+ with `wasm32-unknown-unknown` target installed.

You can run unit tests on this via: 

`cargo test`

Once you are happy with the content, you can compile it to wasm via:

```
RUSTFLAGS='-C link-arg=-s' cargo wasm
cp ../../target/wasm32-unknown-unknown/release/cw1_whitelist.wasm .
ls -l cw1_whitelist.wasm
sha256sum cw1_whitelist.wasm
```

Or for a production-ready (optimized) build, run a build command in the
the repository root: https://github.com/CosmWasm/cw-plus#compiling.
