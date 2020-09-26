# CW20 Basic

This is a basic implementation of a cw20 contract. It implements
the [CW20 spec](../../packages/cw20/README.md) and is designed to
be deployed as is, or imported into other contracts to easily build
cw20-compatible tokens with custom logic.

Implements:

- [x] CW20 Base
- [x] Mintable extension
- [x] Allowances extension

## Running this contract

You will need Rust 1.44.1+ with `wasm32-unknown-unknown` target installed.

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
repository root:

```
docker run --rm -v "$(pwd)":/code \
  --mount type=volume,source="cosmwasm_plus_cache",target=/code/target \
  --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
  cosmwasm/workspace-optimizer:0.10.3
```

The optimized contracts are generated in the `artifacts/` directory.

## Importing this contract

You can also import much of the logic of this contract to build another
ERC20-contract, such as a bonding curve, overiding or extending what you
need.

Basically, you just need to write your handle function and import 
`cw20_base::contract::handle_transfer`, etc and dispatch to them.
This allows you to use custom `HandleMsg` and `QueryMsg` with your additional
calls, but then use the underlying implementation for the standard cw20
messages you want to support. The same with `QueryMsg`. You *could* reuse `init`
as it, but it is likely you will want to change it. And it is rather simple.

Look at [`cw20-staking`](../cw20-staking/README.md) for an example of how to "inherit"
all this token functionality and combine it with custom logic.
