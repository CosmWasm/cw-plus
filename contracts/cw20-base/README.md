# CW20 Basic

This is a basic implementation of a cw20 contract. It implements
the [CW20 spec](../../packages/cw20/README.md) and is designed to
be deloyed as is, or imported into other contracts to easily build
cw20-compatible tokens with custom logic.

Implements:

- [x] CW20 Base
- [ ] Mintable extension
- [ ] Allowances extension

## Running this contract

You will need Rust 1.41+ with `wasm32-unknown-unknown` target installed.

You can run unit tests on this via: 

`cargo test`

Once you are happy with the content, you can compile it to wasm via:

`cargo wasm` (local, quick build)

Or for a production-ready (compressed) build, run the following from the
repository root:

```
docker run --rm -v "$(pwd)":/code \
  --mount type=volume,source="cosmwasm_plus_cache",target=/code/target \
  --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
  cosmwasm/rust-optimizer:0.8.0 ./contracts/cw20-base
mv contract.wasm cw20_base.wasm
```