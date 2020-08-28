pub mod allowances;
pub mod contract;
pub mod enumerable;
pub mod msg;
pub mod state;

mod migrations;

#[cfg(all(target_arch = "wasm32", not(feature = "library")))]
cosmwasm_std::create_entry_points_with_migration!(contract);
