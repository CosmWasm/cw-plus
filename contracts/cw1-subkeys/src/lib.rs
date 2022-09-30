/*!
This builds on [`cw1_whitelist`] to provide the first non-trivial solution.
It still works like [`cw1_whitelist`] with a set of admins (typically 1)
which have full control of the account. However, you can then grant
a number of accounts allowances to send native tokens from this account.

This was proposed in Summer 2019 for the Cosmos Hub and resembles the
functionality of ERC20 (allowances and transfer from).

For more information on this contract, please check out the
[README](https://github.com/CosmWasm/cw-plus/blob/main/contracts/cw1-subkeys/README.md).
*/

pub mod contract;
mod error;
pub mod msg;
pub mod state;

pub use crate::error::ContractError;
