/*!
This may truly be the simplest implementation of [CW1](https://github.com/CosmWasm/cw-plus/blob/main/packages/cw1/README.md),
a generalized contract that anybody can call!

For more information on this contract, please check out the
[README](https://github.com/CosmWasm/cw-plus/blob/main/contracts/cw1-whitelist/README.md).
*/

pub mod contract;
pub mod error;
pub mod msg;

pub use crate::error::ContractError;
