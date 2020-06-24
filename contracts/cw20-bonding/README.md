# CW20 Bonding curve

This builds on the [Basic CW20 interface](../../packages/cw20/README.md)
as implemented in [`cw20-base`](../cw20-base/README.md).

This serves three purposes:

* A usable and extensible contract for arbitrary bonding curves
* A demonstration of how to extend `cw20-base` to add extra functionality
* A demonstration of the [Receiver interface]([Basic CW20 interface](../../packages/cw20/README.md#receiver))

## Design

There are two variants - accepting native tokens and accepting cw20 tokens
as input. 

Minting: When the input is sent to the contract (either via `HandleMsg::Buy{}`
with native tokens, or via `HandleMsg::Receive{}` with cw20 tokens),
those tokens remain on the contract and it issues it's own token to the
sender's account.

Burning: We override the burn function to not only burn the requested tokens,
but also release a proper number of the input tokens to the account that burnt
the custom token

Curves: `handle` specifies a bonding function, which is sent to parameterize
`handle_fn` (which does all the work). The curve is set when compiling
 the contract. In fact many contracts can just wrap `cw20-bonding` and
 specify the custom curve parameter.
 
Read more about [bonding curve math here](https://yos.io/2018/11/10/bonding-curves/)

### Math

**TODO**

## Running this contract

You will need Rust 1.41+ with `wasm32-unknown-unknown` target installed.

You can run unit tests on this via: 

`cargo test`

Once you are happy with the content, you can compile it to wasm via:

```
RUSTFLAGS='-C link-arg=-s' cargo wasm
cp ../../target/wasm32-unknown-unknown/release/cw20_bonding.wasm .
ls -l cw20_bonding.wasm
sha256sum cw20_bonding.wasm
```

Or for a production-ready (compressed) build, run the following from the
repository root (not currently working with this monorepo...)

```
docker run --rm -v "$(pwd)":/code \
  --mount type=volume,source="cosmwasm_plus_cache",target=/code/target \
  --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
  cosmwasm/rust-optimizer:0.8.0 ./contracts/cw20-bonding
mv contract.wasm cw20_bonding.wasm
```

## Importing this contract

You will likely want to import this contract so you can specify your
own bonding curve in your custom contract. That should be relatively easy
to do.

**TODO**