/*!
This may be the simplest implementation of [CW1](https://github.com/CosmWasm/cw-plus/blob/main/packages/cw1/README.md), a whitelist of addresses.
It contains a set of admins that are defined upon creation.
Any of those admins may `Execute` any message via the contract,
per the CW1 spec.

To make this slighly less minimalistic, you can allow the admin set
to be mutable or immutable. If it is mutable, then any admin may
(a) change the admin set and (b) freeze it (making it immutable).

While largely an example contract for CW1, this has various real-world use-cases,
such as a common account that is shared among multiple trusted devices,
or trading an entire account (used as 1 of 1 mutable). Most of the time,
this can be used as a framework to build your own,
more advanced cw1 implementations.

For more information on this contract, please check out the
[README](https://github.com/CosmWasm/cw-plus/blob/main/contracts/cw1-whitelist/README.md).
*/

pub mod contract;
pub mod error;
#[cfg(test)]
mod integration_tests;
pub mod msg;
pub mod state;

pub use crate::error::ContractError;
