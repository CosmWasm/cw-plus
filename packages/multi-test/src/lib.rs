//! Multitest is a design to simulate a blockchain environment in pure Rust.
//! This allows us to run unit tests that involve contract -> contract,
//! and contract -> bank interactions. This is not intended to be a full blockchain app
//! but to simulate the Cosmos SDK x/wasm module close enough to gain confidence in
//! multi-contract deployements before testing them on a live blockchain.
//!
//! To understand the design of this module, please refer to `../DESIGN.md`

mod app;
mod bank;
#[allow(clippy::type_complexity)]
mod contracts;
pub mod custom_handler;
pub mod error;
mod executor;
mod module;
mod staking;
mod test_helpers;
mod transactions;
mod untyped_msg;
mod wasm;

pub use crate::app::{
    custom_app, next_block, App, AppBuilder, BasicApp, BasicAppBuilder, CosmosRouter, Router,
    SudoMsg,
};
pub use crate::bank::{Bank, BankKeeper, BankSudo};
pub use crate::contracts::{Contract, ContractWrapper};
pub use crate::executor::{AppResponse, Executor};
pub use crate::module::Module;
pub use crate::staking::{FailingDistribution, FailingStaking, Staking, StakingSudo};
pub use crate::wasm::{Wasm, WasmKeeper, WasmSudo};
