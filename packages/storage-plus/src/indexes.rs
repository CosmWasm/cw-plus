// this module requires iterator to be useful at all
#![cfg(feature = "iterator")]

use serde::de::DeserializeOwned;
use serde::Serialize;

use cosmwasm_std::{StdResult, Storage};

use crate::U32Key;

pub fn index_string(data: &str) -> Vec<u8> {
    data.as_bytes().to_vec()
}

pub fn index_tuple(name: &str, age: u32) -> (Vec<u8>, U32Key) {
    (index_string(name), U32Key::new(age))
}

pub fn index_triple(name: &str, age: u32, pk: Vec<u8>) -> (Vec<u8>, U32Key, Vec<u8>) {
    (index_string(name), U32Key::new(age), pk)
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
