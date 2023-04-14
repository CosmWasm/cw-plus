/*!
This is a second implementation of the [cw4 spec](https://github.com/CosmWasm/cw-plus/blob/main/packages/cw4/README.md).
It fulfills all elements of the spec, including the raw query lookups,
and is designed to be used as a backing storage for
[cw3 compliant contracts](https://github.com/CosmWasm/cw-plus/blob/main/packages/cw3/README.md).

It provides a similar API to [`cw4-group`](https://github.com/CosmWasm/cw-plus/blob/main/contracts/cw4-group/README.md)
(which handles elected membership),
but rather than appointing members (by admin or multisig), their
membership and weight are based on the number of tokens they have staked.
This is similar to many DAOs.

Only one denom can be bonded with both `min_bond` as the minimum amount
that must be sent by one address to enter, as well as `tokens_per_weight`,
which can be used to normalize the weight (eg. if the token is uatom
and you want 1 weight per ATOM, you can set `tokens_per_weight = 1_000_000`).

There is also an unbonding period (`Duration`) which sets how long the
tokens are frozen before being released. These frozen tokens can neither
be used for voting, nor claimed by the original owner. Only after the period
can you get your tokens back. This liquidity loss is the "skin in the game"
provided by staking to this contract.

For more information on this contract, please check out the
[README](https://github.com/CosmWasm/cw-plus/blob/main/contracts/cw4-stake/README.md).
*/

pub mod contract;
mod error;
pub mod msg;
pub mod state;

pub use crate::error::ContractError;
