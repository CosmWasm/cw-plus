pub mod contract;
pub mod msg;
pub mod state;

#[cfg(all(target_arch = "wasm32", feature = "entrypoint"))]
cosmwasm_std::create_entry_points!(contract);
