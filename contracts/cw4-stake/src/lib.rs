pub mod contract;
pub mod error;
pub mod msg;
pub mod state;

// comment this out and use the lower form if the contract supports migrations
#[cfg(all(target_arch = "wasm32", not(feature = "library")))]
cosmwasm_std::create_entry_points!(contract);
