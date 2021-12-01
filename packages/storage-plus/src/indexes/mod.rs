// this module requires iterator to be useful at all
#![cfg(feature = "iterator")]
mod multi;
mod unique;

pub use multi::MultiIndex;
pub use unique::UniqueIndex;

use serde::de::DeserializeOwned;
use serde::Serialize;

use cosmwasm_std::{StdResult, Storage};

pub fn index_string(data: &str) -> Vec<u8> {
    data.as_bytes().to_vec()
}

pub fn index_tuple(name: &str, age: u32) -> (Vec<u8>, u32) {
    (index_string(name), age)
}

pub fn index_triple(name: &str, age: u32, pk: Vec<u8>) -> (Vec<u8>, u32, Vec<u8>) {
    (index_string(name), age, pk)
}

pub fn index_string_tuple(data1: &str, data2: &str) -> (Vec<u8>, Vec<u8>) {
    (index_string(data1), index_string(data2))
}

// Note: we cannot store traits with generic functions inside `Box<dyn Index>`,
// so I pull S: Storage to a top-level
pub trait Index<T>
where
    T: Serialize + DeserializeOwned + Clone,
{
    fn save(&self, store: &mut dyn Storage, pk: &[u8], data: &T) -> StdResult<()>;
    fn remove(&self, store: &mut dyn Storage, pk: &[u8], old_data: &T) -> StdResult<()>;
}
