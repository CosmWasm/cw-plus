use cosmwasm_std::{StdError, StdResult};

pub trait Deserializable {
    type Output: Sized;

    fn from_slice(value: &[u8]) -> StdResult<Self::Output>;
}

impl Deserializable for String {
    type Output = String;

    fn from_slice(value: &[u8]) -> StdResult<Self::Output> {
        String::from_utf8(value.to_vec())
            // FIXME: Add and use StdError utf-8 error From helper
            .map_err(|err| StdError::generic_err(err.to_string()))
    }
}
