use cosmwasm_std::{WasmMsg, WasmQuery};
use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum Error {
    #[error("Empty attribute key. Value: {value}")]
    EmptyAttributeKey { value: String },

    #[error("Empty attribute value. Key: {key}")]
    EmptyAttributeValue { key: String },

    #[error("Attribute key strats with reserved prefix _: {0}")]
    ReservedAttributeKey(String),

    #[error("Event type too short: {0}")]
    EventTypeTooShort(String),

    #[error("Unsupported wasm query: {0:?}")]
    UnsupportedWasmQuery(WasmQuery),

    #[error("Unsupported wasm message: {0:?}")]
    UnsupportedWasmMsg(WasmMsg),

    #[error("Unregistered code id")]
    UnregisteredCodeId(usize),
}

impl Error {
    pub fn empty_attribute_key(value: impl Into<String>) -> Self {
        Self::EmptyAttributeKey {
            value: value.into(),
        }
    }

    pub fn empty_attribute_value(key: impl Into<String>) -> Self {
        Self::EmptyAttributeValue { key: key.into() }
    }

    pub fn reserved_attribute_key(key: impl Into<String>) -> Self {
        Self::ReservedAttributeKey(key.into())
    }

    pub fn event_type_too_short(ty: impl Into<String>) -> Self {
        Self::EventTypeTooShort(ty.into())
    }
}
