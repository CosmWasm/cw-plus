// Unused code and unimplemented trait methods are useful in
// the next phase of development, so we have decided to keep
// them there. So, instead of litter code with dead-code warning
// suppress instruction, we have put one at the crate level.
// Once phase 2 is complete, this will be removed.
#![allow(dead_code)]

mod block_import_wrapper;
mod block_processor;
mod client;
mod common;
mod db;
mod genesis;
mod grandpa_block_import;
mod justification;
mod light_state;
mod storage;
mod types;
mod verifier;

pub mod contract;
pub use contract::msg;

/// WASM methods exposed to be used by CosmWasm handler
/// All methods are thin wrapper around actual contract contained in
/// contract module.

#[cfg(target_arch = "wasm32")]
pub use wasm::{handle, init, query};

#[cfg(target_arch = "wasm32")]
mod wasm {
    use super::contract;
    use cosmwasm_std::{
        do_handle, do_init, do_query, ExternalApi, ExternalQuerier, ExternalStorage,
    };

    /// WASM Entry point for contract::init
    #[no_mangle]
    pub extern "C" fn init(env_ptr: u32, msg_ptr: u32) -> u32 {
        do_init(
            &contract::init::<ExternalStorage, ExternalApi, ExternalQuerier>,
            env_ptr,
            msg_ptr,
        )
    }

    /// WASM Entry point for contract::handle
    #[no_mangle]
    pub extern "C" fn handle(env_ptr: u32, msg_ptr: u32) -> u32 {
        do_handle(
            &contract::handle::<ExternalStorage, ExternalApi, ExternalQuerier>,
            env_ptr,
            msg_ptr,
        )
    }

    /// WASM Entry point for contract::query
    #[no_mangle]
    pub extern "C" fn query(msg_ptr: u32) -> u32 {
        do_query(
            &contract::query::<ExternalStorage, ExternalApi, ExternalQuerier>,
            msg_ptr,
        )
    }
}
