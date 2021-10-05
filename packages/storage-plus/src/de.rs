use std::array::TryFromSliceError;
use std::convert::TryInto;

use cosmwasm_std::{Addr, StdError, StdResult};

use crate::keys::{IntKey, TimestampKey};

pub trait KeyDeserialize {
    type Output: Sized;

    fn from_vec(value: Vec<u8>) -> StdResult<Self::Output>;

    fn from_slice(value: &[u8]) -> StdResult<Self::Output> {
        Self::from_vec(value.to_vec())
    }
}

impl KeyDeserialize for () {
    type Output = ();

    #[inline(always)]
    fn from_vec(_value: Vec<u8>) -> StdResult<Self::Output> {
        Ok(())
    }
}

impl KeyDeserialize for Vec<u8> {
    type Output = Vec<u8>;

    #[inline(always)]
    fn from_vec(value: Vec<u8>) -> StdResult<Self::Output> {
        Ok(value)
    }
}

impl KeyDeserialize for &Vec<u8> {
    type Output = Vec<u8>;

    #[inline(always)]
    fn from_vec(value: Vec<u8>) -> StdResult<Self::Output> {
        Ok(value)
    }
}

impl KeyDeserialize for &[u8] {
    type Output = Vec<u8>;

    #[inline(always)]
    fn from_vec(value: Vec<u8>) -> StdResult<Self::Output> {
        Ok(value)
    }
}

impl KeyDeserialize for String {
    type Output = String;

    #[inline(always)]
    fn from_vec(value: Vec<u8>) -> StdResult<Self::Output> {
        String::from_utf8(value).map_err(StdError::invalid_utf8)
    }
}

impl KeyDeserialize for &String {
    type Output = String;

    #[inline(always)]
    fn from_vec(value: Vec<u8>) -> StdResult<Self::Output> {
        Self::Output::from_vec(value)
    }
}

impl KeyDeserialize for &str {
    type Output = String;

    #[inline(always)]
    fn from_vec(value: Vec<u8>) -> StdResult<Self::Output> {
        Self::Output::from_vec(value)
    }
}

impl KeyDeserialize for Addr {
    type Output = Addr;

    #[inline(always)]
    fn from_vec(value: Vec<u8>) -> StdResult<Self::Output> {
        Ok(Addr::unchecked(String::from_vec(value)?))
    }
}

impl KeyDeserialize for &Addr {
    type Output = Addr;

    #[inline(always)]
    fn from_vec(value: Vec<u8>) -> StdResult<Self::Output> {
        Self::Output::from_vec(value)
    }
}

macro_rules! integer_de {
    (for $($t:ty),+) => {
        $(impl KeyDeserialize for IntKey<$t> {
            type Output = $t;

            #[inline(always)]
            fn from_vec(value: Vec<u8>) -> StdResult<Self::Output> {
                Ok(<$t>::from_be_bytes(value.as_slice().try_into()
                    .map_err(|err: TryFromSliceError| StdError::generic_err(err.to_string()))?))
            }
        })*
    }
}

integer_de!(for i8, u8, i16, u16, i32, u32, i64, u64, i128, u128);

impl KeyDeserialize for TimestampKey {
    type Output = u64;

    #[inline(always)]
    fn from_vec(value: Vec<u8>) -> StdResult<Self::Output> {
        <IntKey<Self::Output>>::from_vec(value)
    }
}

fn parse_length(value: &[u8]) -> StdResult<usize> {
    Ok(u16::from_be_bytes(
        value
            .try_into()
            .map_err(|_| StdError::generic_err("Could not read 2 byte length"))?,
    )
    .into())
}

impl<T: KeyDeserialize, U: KeyDeserialize> KeyDeserialize for (T, U) {
    type Output = (T::Output, U::Output);

    #[inline(always)]
    fn from_vec(mut value: Vec<u8>) -> StdResult<Self::Output> {
        let mut tu = value.split_off(2);
        let t_len = parse_length(&value)?;
        let u = tu.split_off(t_len);

        Ok((T::from_vec(tu)?, U::from_vec(u)?))
    }
}

impl<T: KeyDeserialize, U: KeyDeserialize, V: KeyDeserialize> KeyDeserialize for (T, U, V) {
    type Output = (T::Output, U::Output, V::Output);

    #[inline(always)]
    fn from_vec(mut value: Vec<u8>) -> StdResult<Self::Output> {
        let mut tuv = value.split_off(2);
        let t_len = parse_length(&value)?;
        let mut len_uv = tuv.split_off(t_len);

        let mut uv = len_uv.split_off(2);
        let u_len = parse_length(&len_uv)?;
        let v = uv.split_off(u_len);

        Ok((T::from_vec(tuv)?, U::from_vec(uv)?, V::from_vec(v)?))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{PrimaryKey, U32Key};

    const BYTES: &[u8] = b"Hello";
    const STRING: &str = "Hello";

    #[test]
    #[allow(clippy::unit_cmp)]
    fn deserialize_empty_works() {
        assert_eq!(<()>::from_slice(BYTES).unwrap(), ());
    }

    #[test]
    fn deserialize_bytes_works() {
        assert_eq!(<Vec<u8>>::from_slice(BYTES).unwrap(), BYTES);
        assert_eq!(<&Vec<u8>>::from_slice(BYTES).unwrap(), BYTES);
        assert_eq!(<&[u8]>::from_slice(BYTES).unwrap(), BYTES);
    }

    #[test]
    fn deserialize_string_works() {
        assert_eq!(<String>::from_slice(BYTES).unwrap(), STRING);
        assert_eq!(<&String>::from_slice(BYTES).unwrap(), STRING);
        assert_eq!(<&str>::from_slice(BYTES).unwrap(), STRING);
    }

    #[test]
    fn deserialize_broken_string_errs() {
        assert!(matches!(
            <String>::from_slice(b"\xc3").err(),
            Some(StdError::InvalidUtf8 { .. })
        ));
    }

    #[test]
    fn deserialize_addr_works() {
        assert_eq!(<Addr>::from_slice(BYTES).unwrap(), Addr::unchecked(STRING));
        assert_eq!(<&Addr>::from_slice(BYTES).unwrap(), Addr::unchecked(STRING));
    }

    #[test]
    fn deserialize_broken_addr_errs() {
        assert!(matches!(
            <Addr>::from_slice(b"\xc3").err(),
            Some(StdError::InvalidUtf8 { .. })
        ));
    }

    #[test]
    fn deserialize_integer_works() {
        assert_eq!(<IntKey<u8>>::from_slice(&[1]).unwrap(), 1u8);
        assert_eq!(<IntKey<i8>>::from_slice(&[128]).unwrap(), -1i8 << 7);
        assert_eq!(<IntKey<u16>>::from_slice(&[1, 0]).unwrap(), 1u16 << 8);
        assert_eq!(
            <IntKey<i16>>::from_slice(&[128, 0]).unwrap(),
            -1i16 << (8 + 7)
        );
        assert_eq!(
            <IntKey<u32>>::from_slice(&[1, 0, 0, 0]).unwrap(),
            1u32 << (3 * 8)
        );
        assert_eq!(
            <IntKey<i32>>::from_slice(&[128, 0, 0, 0]).unwrap(),
            -1i32 << (3 * 8 + 7)
        );
        assert_eq!(
            <IntKey<u64>>::from_slice(&[1, 0, 0, 0, 0, 0, 0, 0]).unwrap(),
            1u64 << (7 * 8)
        );
        assert_eq!(
            <IntKey<i64>>::from_slice(&[128, 0, 0, 0, 0, 0, 0, 0]).unwrap(),
            -1i64 << (7 * 8 + 7)
        );
        assert_eq!(
            <IntKey<u128>>::from_slice(&[1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]).unwrap(),
            1u128 << (15 * 8)
        );
        assert_eq!(
            <IntKey<i128>>::from_slice(&[
                255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255
            ])
            .unwrap(),
            -1i128
        );
    }

    #[test]
    fn deserialize_broken_integer_errs() {
        // One byte less fails
        assert!(matches!(
            <IntKey<u16>>::from_slice(&[1]).err(),
            Some(StdError::GenericErr { .. })
        ));

        // More bytes fails too
        assert!(matches!(
            <IntKey<u8>>::from_slice(&[1, 2]).err(),
            Some(StdError::GenericErr { .. })
        ));
    }

    #[test]
    fn deserialize_timestamp_works() {
        assert_eq!(
            <TimestampKey>::from_slice(&[1, 0, 0, 0, 0, 0, 0, 0]).unwrap(),
            1u64 << (7 * 8)
        );
    }

    #[test]
    fn deserialize_broken_timestamp_errs() {
        // More bytes fails
        assert!(matches!(
            <TimestampKey>::from_slice(&[1, 2, 3, 4, 5, 6, 7, 8, 9]).err(),
            Some(StdError::GenericErr { .. })
        ));
    }

    #[test]
    fn deserialize_tuple_works() {
        assert_eq!(
            <(&[u8], &str)>::from_slice((BYTES, STRING).joined_key().as_slice()).unwrap(),
            (BYTES.to_vec(), STRING.to_string())
        );
    }

    #[test]
    fn deserialize_triple_works() {
        assert_eq!(
            <(&[u8], U32Key, &str)>::from_slice(
                (BYTES, U32Key::new(1234), STRING).joined_key().as_slice()
            )
            .unwrap(),
            (BYTES.to_vec(), 1234, STRING.to_string())
        );
    }
}
