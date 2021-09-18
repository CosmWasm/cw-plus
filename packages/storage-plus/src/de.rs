use crate::keys::{IntKey, TimestampKey};
use cosmwasm_std::{Addr, StdError, StdResult};
use std::array::TryFromSliceError;
use std::convert::TryInto;

pub trait Deserializable {
    type Output: Sized;

    fn from_slice(value: &[u8]) -> StdResult<Self::Output>;
}

impl Deserializable for () {
    type Output = ();

    fn from_slice(_value: &[u8]) -> StdResult<Self::Output> {
        Ok(())
    }
}

macro_rules! bytes_de {
    (for $($t:ty),+) => {
        $(impl Deserializable for $t {
            type Output = Vec<u8>;

            fn from_slice(value: &[u8]) -> StdResult<Self::Output> {
                Ok(value.to_vec())
            }
        })*
    }
}

bytes_de!(for Vec<u8>, &Vec<u8>, [u8], &[u8]);

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

string_de!(for String, &String, str, &str, Addr, &Addr);

macro_rules! integer_de {
    (for $($t:ty),+) => {
        $(impl Deserializable for IntKey<$t> {
            type Output = $t;

            fn from_slice(value: &[u8]) -> StdResult<Self::Output> {
                Ok(<$t>::from_be_bytes(value.try_into()
                    // FIXME: Add and use StdError try-from error From helper
                    .map_err(|err: TryFromSliceError| StdError::generic_err(err.to_string()))?))
            }
        })*
    }
}

integer_de!(for i8, u8, i16, u16, i32, u32, i64, u64, i128, u128);

impl Deserializable for TimestampKey {
    type Output = u64;

    fn from_slice(value: &[u8]) -> StdResult<Self::Output> {
        Ok(<u64>::from_be_bytes(
            value
                .try_into()
                // FIXME: Add and use StdError try-from error From helper
                .map_err(|err: TryFromSliceError| StdError::generic_err(err.to_string()))?,
        ))
    }
}

impl<T: Deserializable, U: Deserializable> Deserializable for (T, U) {
    type Output = (T::Output, U::Output);

    fn from_slice(value: &[u8]) -> StdResult<Self::Output> {
        let t_len = u16::from_be_bytes(
            value[..2]
                .try_into()
                // FIXME: Add and use StdError try-from error From helper
                .map_err(|err: TryFromSliceError| StdError::generic_err(err.to_string()))?,
        ) as usize;
        let t = T::from_slice(&value[2..2 + t_len])?;
        let u = U::from_slice(&value[2 + t_len..])?;

        Ok((t, u))
    }
}

impl<T: Deserializable, U: Deserializable, V: Deserializable> Deserializable for (T, U, V) {
    type Output = (T::Output, U::Output, V::Output);

    fn from_slice(value: &[u8]) -> StdResult<Self::Output> {
        let t_len = u16::from_be_bytes(
            value[..2]
                .try_into()
                // FIXME: Add and use StdError try-from error From helper
                .map_err(|err: TryFromSliceError| StdError::generic_err(err.to_string()))?,
        ) as usize;
        let t = T::from_slice(&value[2..2 + t_len])?;
        let u_len = u16::from_be_bytes(
            value[2 + t_len..4 + t_len]
                .try_into()
                // FIXME: Add and use StdError try-from error From helper
                .map_err(|err: TryFromSliceError| StdError::generic_err(err.to_string()))?,
        ) as usize;
        let u = U::from_slice(&value[4 + t_len..4 + t_len + u_len])?;
        let v = V::from_slice(&value[4 + t_len + u_len..])?;

        Ok((t, u, v))
    }
}
