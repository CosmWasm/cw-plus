use cosmwasm_std::{Addr, StdError, StdResult};

pub trait Deserializable {
    type Output: Sized;

    fn from_slice(value: &[u8]) -> StdResult<Self::Output>;
}

macro_rules! string_de {
    (for $($t:ty),+) => {
        $(impl Deserializable for $t {
            type Output = String;

            fn from_slice(value: &[u8]) -> StdResult<Self::Output> {
                // FIXME?: Use `from_utf8_unchecked` for String, &str
                String::from_utf8(value.to_vec())
                    // FIXME: Add and use StdError utf-8 error From helper
                    .map_err(|err| StdError::generic_err(err.to_string()))
    }
        })*
    }
}

// TODO: Confirm / extend these
string_de!(for String, &str, &[u8], Addr, &Addr);
