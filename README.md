# CosmWasm Plus

[![CircleCI](https://circleci.com/gh/CosmWasm/cosmwasm-plus/tree/master.svg?style=shield)](https://circleci.com/gh/CosmWasm/cosmwasm-plus/tree/master)

| Specification    | Download                                                                                                   | Docs                                                            |
| ---------------- | ---------------------------------------------------------------------------------------------------------  | ----------------------------------------------------------------|
| cw0              | [![cw0 on crates.io](https://img.shields.io/crates/v/cw0.svg)](https://crates.io/crates/cw0)              | [![Docs](https://docs.rs/cw0/badge.svg)](https://docs.rs/cw0)    |
| cw1              | [![cw1 on crates.io](https://img.shields.io/crates/v/cw1.svg)](https://crates.io/crates/cw1)              | [![Docs](https://docs.rs/cw1/badge.svg)](https://docs.rs/cw1)    |
| cw2              | [![cw2 on crates.io](https://img.shields.io/crates/v/cw2.svg)](https://crates.io/crates/cw2)              | [![Docs](https://docs.rs/cw2/badge.svg)](https://docs.rs/cw2)    |
| cw20              | [![cw20 on crates.io](https://img.shields.io/crates/v/cw20.svg)](https://crates.io/crates/cw20)              | [![Docs](https://docs.rs/cw20/badge.svg)](https://docs.rs/cw20)    |
| cw721              | [![cw721 on crates.io](https://img.shields.io/crates/v/cw721.svg)](https://crates.io/crates/cw721)              | [![Docs](https://docs.rs/cw721/badge.svg)](https://docs.rs/cw721)    |

| Contracts               | Download                                                                                                                      | Docs                                                                     |
| ----------------------- | ----------------------------------------------------------------------------------------------------------------------------  | -------------------------------------------------------------------------|
| cw1-subkeys             | [![cw1-subkeys on crates.io](https://img.shields.io/crates/v/cw1-subkeys.svg)](https://crates.io/crates/cw1-subkeys)          | [![Docs](https://docs.rs/cw1-subkeys/badge.svg)](https://docs.rs/cw1-subkeys)    |
| cw1-whitelist           | [![cw1-whitelist on crates.io](https://img.shields.io/crates/v/cw1-whitelist.svg)](https://crates.io/crates/cw1-whitelist)          | [![Docs](https://docs.rs/cw1-whitelist/badge.svg)](https://docs.rs/cw1-whitelist)    |
| cw20-atomic-swap                | [![cw20-atomic-swap on crates.io](https://img.shields.io/crates/v/cw20-atomic-swap.svg)](https://crates.io/crates/cw20-atomic-swap)          | [![Docs](https://docs.rs/cw20-atomic-swap/badge.svg)](https://docs.rs/cw20-atomic-swap)    |
| cw20-base                | [![cw20-base on crates.io](https://img.shields.io/crates/v/cw20-base.svg)](https://crates.io/crates/cw20-base)          | [![Docs](https://docs.rs/cw20-base/badge.svg)](https://docs.rs/cw20-base)    |
| cw20-escrow             | [![cw20-escrow on crates.io](https://img.shields.io/crates/v/cw20-escrow.svg)](https://crates.io/crates/cw20-escrow)          | [![Docs](https://docs.rs/cw20-escrow/badge.svg)](https://docs.rs/cw20-escrow)    |
| cw20-staking             | [![cw20-staking on crates.io](https://img.shields.io/crates/v/cw20-staking.svg)](https://crates.io/crates/cw20-staking)          | [![Docs](https://docs.rs/cw20-staking/badge.svg)](https://docs.rs/cw20-staking)    |


This is a collection of specification and contracts designed for
use on real networks. They are designed not just as examples, but to
solve real-world use cases, and to provide a reusable basis to build 
many custom contracts.

If you don't know what CosmWasm is, please check out 
[our homepage](https://cosmwasm.com) and 
[our documentation](https://docs.cosmwasm.com) to get more background.
We are running a [public testnet](https://github.com/CosmWasm/testnets/blob/master/coralnet/README.md)
you can use to test out any contracts.

**Warning** None of these contracts have been audited and no liability is
assumed for the use of any of this code. They are provided to turbo-start
your projects.

**Note** All code in pre-1.0 packages is in "draft" form, meaning it may
undergo minor changes and additions until 1.0. For example between 0.1 and
0.2 we adjusted the `Expiration` type to make the JSON representation 
cleaner (before: `expires: {at_height: {height: 12345}}` after 
`expires: {at_height: 12345}`)

## Specifications

The most reusable components are the various cwXYZ specifications under
`packages`. Each one defines a standard interface for different domains,
eg. [cw20](./packages/cw20/README.md) for fungible tokens, 
[cw721](./packages/cw721/README.md) for non-fungible tokens, 
[cw1](./packages/cw1/README.md) for  "proxy contracts", etc. 
The interface comes with a human description in the READMEs, as well
as Rust types that can be imported.

They contain no logic, but specify an interface. It shows what you
need to implement to create a compatible contracts, as well as what
interface we guarantee to any consumer of such contracts. This is
the real bonus of specifications, we can create an escrow contract that
can handle many different fungible tokens, as long as they all adhere to
the cw20 specification.

If you have ideas for new specifications or want to make enhancements to
existing spec, please [raise an issue](https://github.com/CosmWasm/cosmwasm-plus/issues)
or [create a pull request](https://github.com/CosmWasm/cosmwasm-plus/pulls) on this repo.

## Contracts

We provide sample contracts that either implement or consume these 
specifications to both provide examples, as well as provide a basis
for code you can extend for more custom contacts, without worrying
about reinventing the wheel each time.
For example [`cw20-base`](./contracts/cw20-base) is a basic implementation
of a `cw20` compatible contract that can be imported in any custom 
contract you want to build on it. 

CW1 Proxy Contracts:

* [`cw1-whitelist`](./contracts/cw1-whitelist) a minimal implementation of `cw1`
mainly designed for reference
* [`cw1-subkeys`](./contracts/cw1-subkeys) a simple, but useful implementation,
which lets us use a proxy contract to provide "allowances" for native tokens
without modifying the `bank` module

CW20 Fungible Tokens:

* [`cw20-base`](./contracts/cw20-base) a straightforward, but complete
implementation of the cw20 spec along with all extensions. Can be deployed
as-is, or imported by other contracts
* [`cw20-staking`](./contracts/cw20-staking) provides staking derivatives,
staking native tokens on your behalf and minting cw20 tokens that can
be used to claim them. It uses `cw20-base` for all the cw20 logic and
only implements the interactions with the staking module and accounting
for prices
* [`cw20-escrow`](./contracts/cw20-escrow) is a basic escrow contract 
(arbiter can release or refund tokens) that is compatible with all native
and cw20 tokens. This is a good example to show how to interact with
cw20 tokens.

## Licenses

This repo contains two license, [Apache 2.0](./LICENSE-APACHE) and
[AGPL 3.0](./LICENSE-AGPL.md). All crates in this repo may be licensed
as one or the other. Please check the `NOTICE` in each crate or the 
relevant `Cargo.toml` file for clarity.

All *specifications* will always be Apache-2.0. All contracts that are
meant to be *building blocks* will also be Apache-2.0. This is along
the lines of Open Zepellin or other public references.

Contracts that are "ready to deploy" may be licensed under AGPL 3.0 to 
encourage anyone using them to contribute back any improvements they
make. This is common practice for actual projects running on Ethereum,
like Uniswap or Maker DAO.

