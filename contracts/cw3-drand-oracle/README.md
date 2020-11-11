# Rand – A drand client as a CosmWasm smart contract

In drand, random beacons are distributed via HTTP, Gossipsub, Tor or Twitter. Such network sources cannot be accessed by a blockchain directly. However, through this CosmWasm smart contract which allows storing random beacons on chain. Using cross-contract queries, other contracts can then read those random values and use them in their logic.

## Development build

Some fast checks

```sh
cargo fmt && cargo unit-test && cargo check --tests && cargo schema && cargo clippy -- -D warnings
```

## Production build

```
docker run --rm -v "$(pwd)":/code \
  --mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
  --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
  cosmwasm/rust-optimizer:0.10.4
```

## Run in singlepass

In order to measure gas consumption, singlepass tests need to be used. E.g.

```sh
cargo wasm
cargo +nightly integration-test --no-default-features --features singlepass verify_valid -- --nocapture
```
```
