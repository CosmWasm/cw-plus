#![cfg(test)]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cw_storage_plus::Item;

pub mod contracts;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EmptyMsg {}

/// This is just a demo place so we can test custom message handling
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename = "snake_case")]
pub enum CustomMsg {
    SetName { name: String },
    SetAge { age: u32 },
}

const COUNT: Item<u32> = Item::new("count");
