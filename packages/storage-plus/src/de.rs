use std::array::TryFromSliceError;
use std::convert::TryInto;

use cosmwasm_std::{Addr, StdError, StdResult};

use crate::int_key::CwIntKey;

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
        $(impl KeyDeserialize for $t {
            type Output = $t;

            #[inline(always)]
            fn from_vec(value: Vec<u8>) -> StdResult<Self::Output> {
                Ok(<$t>::from_cw_bytes(value.as_slice().try_into()
                    .map_err(|err: TryFromSliceError| StdError::generic_err(err.to_string()))?))
            }
        })*
    }
}

integer_de!(for i8, u8, i16, u16, i32, u32, i64, u64, i128, u128);

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
    use crate::PrimaryKey;

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
    fn deserialize_naked_integer_works() {
        assert_eq!(u8::from_slice(&[1]).unwrap(), 1u8);
        assert_eq!(i8::from_slice(&[127]).unwrap(), -1i8);
        assert_eq!(i8::from_slice(&[128]).unwrap(), 0i8);

        assert_eq!(u16::from_slice(&[1, 0]).unwrap(), 256u16);
        assert_eq!(i16::from_slice(&[128, 0]).unwrap(), 0i16);
        assert_eq!(i16::from_slice(&[127, 255]).unwrap(), -1i16);

        assert_eq!(u32::from_slice(&[1, 0, 0, 0]).unwrap(), 16777216u32);
        assert_eq!(i32::from_slice(&[128, 0, 0, 0]).unwrap(), 0i32);
        assert_eq!(i32::from_slice(&[127, 255, 255, 255]).unwrap(), -1i32);

        assert_eq!(
            u64::from_slice(&[1, 0, 0, 0, 0, 0, 0, 0]).unwrap(),
            72057594037927936u64
        );
        assert_eq!(i64::from_slice(&[128, 0, 0, 0, 0, 0, 0, 0]).unwrap(), 0i64);
        assert_eq!(
            i64::from_slice(&[127, 255, 255, 255, 255, 255, 255, 255]).unwrap(),
            -1i64
        );

        assert_eq!(
            u128::from_slice(&[1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]).unwrap(),
            1329227995784915872903807060280344576u128
        );
        assert_eq!(
            i128::from_slice(&[128, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]).unwrap(),
            0i128
        );
        assert_eq!(
            i128::from_slice(&[
                127, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255
            ])
            .unwrap(),
            -1i128
        );
        assert_eq!(
            i128::from_slice(&[
                255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255
            ])
            .unwrap(),
            170141183460469231731687303715884105727i128,
        );
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
            <(&[u8], u32, &str)>::from_slice((BYTES, 1234u32, STRING).joined_key().as_slice())
                .unwrap(),
            (BYTES.to_vec(), 1234, STRING.to_string())
        );
    }
}
