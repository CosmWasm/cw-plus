use std::array::TryFromSliceError;
use std::convert::TryInto;

use cosmwasm_std::{StdError, StdResult};

use crate::de::KeyDeserialize;
use crate::keys_old::IntKeyOld;

macro_rules! intkey_old_de {
    (for $($t:ty),+) => {
        $(impl KeyDeserialize for IntKeyOld<$t> {
            type Output = $t;

            #[inline(always)]
            fn from_vec(value: Vec<u8>) -> StdResult<Self::Output> {
                Ok(<$t>::from_be_bytes(value.as_slice().try_into()
                    .map_err(|err: TryFromSliceError| StdError::generic_err(err.to_string()))?))
            }
        })*
    }
}

intkey_old_de!(for i8, u8, i16, u16, i32, u32, i64, u64, i128, u128);

#[cfg(test)]
mod test {
    use super::*;
    use crate::keys_old::IntKeyOld;

    #[test]
    fn deserialize_integer_old_works() {
        assert_eq!(<IntKeyOld<u8>>::from_slice(&[1]).unwrap(), 1u8);
        assert_eq!(<IntKeyOld<i8>>::from_slice(&[127]).unwrap(), 127i8);
        assert_eq!(<IntKeyOld<i8>>::from_slice(&[128]).unwrap(), -128i8);

        assert_eq!(<IntKeyOld<u16>>::from_slice(&[1, 0]).unwrap(), 256u16);
        assert_eq!(<IntKeyOld<i16>>::from_slice(&[128, 0]).unwrap(), -32768i16);
        assert_eq!(<IntKeyOld<i16>>::from_slice(&[127, 255]).unwrap(), 32767i16);

        assert_eq!(
            <IntKeyOld<u32>>::from_slice(&[1, 0, 0, 0]).unwrap(),
            16777216u32
        );
        assert_eq!(
            <IntKeyOld<i32>>::from_slice(&[128, 0, 0, 0]).unwrap(),
            -2147483648i32
        );
        assert_eq!(
            <IntKeyOld<i32>>::from_slice(&[127, 255, 255, 255]).unwrap(),
            2147483647i32
        );

        assert_eq!(
            <IntKeyOld<u64>>::from_slice(&[1, 0, 0, 0, 0, 0, 0, 0]).unwrap(),
            72057594037927936u64
        );
        assert_eq!(
            <IntKeyOld<i64>>::from_slice(&[128, 0, 0, 0, 0, 0, 0, 0]).unwrap(),
            -9223372036854775808i64
        );
        assert_eq!(
            <IntKeyOld<i64>>::from_slice(&[127, 255, 255, 255, 255, 255, 255, 255]).unwrap(),
            9223372036854775807i64
        );

        assert_eq!(
            <IntKeyOld<u128>>::from_slice(&[1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0])
                .unwrap(),
            1329227995784915872903807060280344576u128
        );
        assert_eq!(
            <IntKeyOld<i128>>::from_slice(&[128, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0])
                .unwrap(),
            -170141183460469231731687303715884105728i128
        );
        assert_eq!(
            <IntKeyOld<i128>>::from_slice(&[
                127, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255
            ])
            .unwrap(),
            170141183460469231731687303715884105727i128
        );
        assert_eq!(
            <IntKeyOld<i128>>::from_slice(&[
                255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255
            ])
            .unwrap(),
            -1i128,
        );
    }

    #[test]
    fn deserialize_broken_integer_old_errs() {
        // One byte less fails
        assert!(matches!(
            <IntKeyOld<u16>>::from_slice(&[1]).err(),
            Some(StdError::GenericErr { .. })
        ));

        // More bytes fails too
        assert!(matches!(
            <IntKeyOld<u8>>::from_slice(&[1, 2]).err(),
            Some(StdError::GenericErr { .. })
        ));
    }
}
