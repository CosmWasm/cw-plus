//! Multitest is a design to simulate a blockchain environment in pure Rust.
//! This allows us to run unit tests that involve contract -> contract,
//! and contract -> bank interactions. This is not intended to be a full blockchain app
//! but to simulate the Cosmos SDK x/wasm module close enough to gain confidence in
//! multi-contract deployements before testing them on a live blockchain.
//!
//! To understand the design of this module, please refer to `../DESIGN.md`

mod app;
mod bank;
mod contracts;
pub mod custom_handler;
pub mod error;
mod executor;
mod test_helpers;
mod transactions;
mod wasm;

pub use crate::app::{next_block, App, AppBuilder, Router};
pub use crate::bank::{Bank, BankKeeper};
pub use crate::contracts::{Contract, ContractWrapper};
pub use crate::custom_handler::CustomHandler;
pub use crate::executor::{AppResponse, Executor};
pub use crate::wasm::{parse_contract_addr, Wasm, WasmKeeper};
