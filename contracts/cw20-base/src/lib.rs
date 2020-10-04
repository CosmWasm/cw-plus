pub mod allowances;
pub mod contract;
pub mod enumerable;
mod error;
pub mod msg;
pub mod state;

mod migrations;

pub use crate::error::ContractError;

#[cfg(all(target_arch = "wasm32", not(feature = "library")))]
cosmwasm_std::create_entry_points_with_migration!(contract);
