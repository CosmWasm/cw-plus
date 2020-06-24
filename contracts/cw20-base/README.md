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

```
RUSTFLAGS='-C link-arg=-s' cargo wasm
cp ../../target/wasm32-unknown-unknown/release/cw20_base.wasm .
ls -l cw20_base.wasm
sha256sum cw20_base.wasm
```

Or for a production-ready (compressed) build, run the following from the
repository root (not currently working with this monorepo...)

```
docker run --rm -v "$(pwd)":/code \
  --mount type=volume,source="cosmwasm_plus_cache",target=/code/target \
  --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
  cosmwasm/rust-optimizer:0.8.0 ./contracts/cw20-base
mv contract.wasm cw20_base.wasm
```

## Importing this contract

You can also import much of the logic of this contract to build another
ERC20-contract, such as a bonding curve, overiding or extending what you
need.

**TODO**