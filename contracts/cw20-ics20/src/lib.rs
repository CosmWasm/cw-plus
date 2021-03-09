pub mod amount;
pub mod contract;
pub mod error;
pub mod ibc;
pub mod msg;
pub mod state;

#[cfg(all(target_arch = "wasm32", not(feature = "library")))]
cosmwasm_std::create_entry_points!(contract);
