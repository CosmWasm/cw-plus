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

## License

```
A drand client in a smart contract for CosmWasm.
Copyright (C) 2020 Confio OÜ

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU Affero General Public License as
published by the Free Software Foundation, either version 3 of the
License, or (at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU Affero General Public License for more details.

You should have received a copy of the GNU Affero General Public License
along with this program.  If not, see <http://www.gnu.org/licenses/>.
```
