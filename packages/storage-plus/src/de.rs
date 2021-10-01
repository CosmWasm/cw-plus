use std::array::TryFromSliceError;
use std::convert::TryInto;

use cosmwasm_std::{Addr, StdError, StdResult};

use crate::keys::{IntKey, TimestampKey};

pub trait KeyDeserialize {
    type Output: Sized;

    fn from_slice(value: &[u8]) -> StdResult<Self::Output>;
}

impl KeyDeserialize for () {
    type Output = ();

    fn from_slice(_value: &[u8]) -> StdResult<Self::Output> {
        Ok(())
    }
}

impl KeyDeserialize for Vec<u8> {
    type Output = Vec<u8>;

    fn from_slice(value: &[u8]) -> StdResult<Self::Output> {
        Ok(value.to_vec())
    }
}

impl KeyDeserialize for &Vec<u8> {
    type Output = Vec<u8>;

    fn from_slice(value: &[u8]) -> StdResult<Self::Output> {
        <Vec<u8>>::from_slice(value)
    }
}

impl KeyDeserialize for &[u8] {
    type Output = Vec<u8>;

    fn from_slice(value: &[u8]) -> StdResult<Self::Output> {
        <Vec<u8>>::from_slice(value)
    }
}

impl KeyDeserialize for String {
    type Output = String;

    fn from_slice(value: &[u8]) -> StdResult<Self::Output> {
        String::from_utf8(value.to_vec())
            // FIXME: Add and use StdError utf-8 error From helper
            .map_err(|err| StdError::generic_err(err.to_string()))
    }
}

impl KeyDeserialize for &String {
    type Output = String;

    fn from_slice(value: &[u8]) -> StdResult<Self::Output> {
        String::from_slice(value)
    }
}

impl KeyDeserialize for &str {
    type Output = String;

    fn from_slice(value: &[u8]) -> StdResult<Self::Output> {
        String::from_slice(value)
    }
}

impl KeyDeserialize for Addr {
    type Output = Addr;

    fn from_slice(value: &[u8]) -> StdResult<Self::Output> {
        Ok(Addr::unchecked(String::from_slice(value)?))
    }
}

impl KeyDeserialize for &Addr {
    type Output = Addr;

    fn from_slice(value: &[u8]) -> StdResult<Self::Output> {
        Addr::from_slice(value)
    }
}

macro_rules! integer_de {
    (for $($t:ty),+) => {
        $(impl KeyDeserialize for IntKey<$t> {
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

impl KeyDeserialize for TimestampKey {
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

impl<T: KeyDeserialize, U: KeyDeserialize> KeyDeserialize for (T, U) {
    type Output = (T::Output, U::Output);

    fn from_slice(value: &[u8]) -> StdResult<Self::Output> {
        let (len, data) = value.split_at(2);
        let t_len = u16::from_be_bytes(
            len.try_into()
                // FIXME: Add and use StdError try-from error From helper
                .map_err(|err: TryFromSliceError| StdError::generic_err(err.to_string()))?,
        ) as usize;
        let (t, u) = data.split_at(t_len);

        Ok((T::from_slice(t)?, U::from_slice(u)?))
    }
}

impl<T: KeyDeserialize, U: KeyDeserialize, V: KeyDeserialize> KeyDeserialize for (T, U, V) {
    type Output = (T::Output, U::Output, V::Output);

    fn from_slice(value: &[u8]) -> StdResult<Self::Output> {
        let (len, data) = value.split_at(2);
        let t_len = u16::from_be_bytes(
            len.try_into()
                // FIXME: Add and use StdError try-from error From helper
                .map_err(|err: TryFromSliceError| StdError::generic_err(err.to_string()))?,
        ) as usize;
        let (t, data) = data.split_at(t_len);

        let (len, data) = data.split_at(2);
        let u_len = u16::from_be_bytes(
            len.try_into()
                // FIXME: Add and use StdError try-from error From helper
                .map_err(|err: TryFromSliceError| StdError::generic_err(err.to_string()))?,
        ) as usize;
        let (u, v) = data.split_at(u_len);

        Ok((T::from_slice(t)?, U::from_slice(u)?, V::from_slice(v)?))
    }
}
