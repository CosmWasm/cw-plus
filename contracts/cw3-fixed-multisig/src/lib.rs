/*!
This is a simple implementation of the [cw3 spec](https://github.com/CosmWasm/cw-plus/blob/main/packages/cw3/README.md).
It is a multisig with a fixed set of addresses created upon instatiation.
Each address may have the same weight (K of N), or some may have extra voting
power. This works much like the native Cosmos SDK multisig, except that rather
than aggregating the signatures off chain and submitting the final result,
we aggregate the approvals on-chain.

This is usable as is, and probably the most secure implementation of cw3
(as it is the simplest), but we will be adding more complex cases, such
as updating the multisig set, different voting rules for the same group
with different permissions, and even allow token-weighted voting. All through
the same client interface.

For more information on this contract, please check out the
[README](https://github.com/CosmWasm/cw-plus/blob/main/contracts/cw3-fixed-multisig/README.md).
*/

pub mod contract;
mod error;
mod integration_tests;
pub mod msg;
pub mod state;

pub use crate::error::ContractError;
