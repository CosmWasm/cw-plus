/*!
This is a basic implementation of a cw1155 contract.
It implements the [CW1155 spec](https://github.com/CosmWasm/cw-plus/blob/main/packages/cw1155/README.md) and manages multiple tokens
(fungible or non-fungible) under one contract.

For more information on this contract, please check out the
[README](https://github.com/CosmWasm/cw-plus/blob/main/contracts/cw1155-base/README.md).
*/

pub mod contract;
mod error;
pub mod execute;
pub mod helpers;
pub mod msg;
pub mod query;
pub mod state;

pub use crate::error::ContractError;

#[cfg(test)]
mod tests;
