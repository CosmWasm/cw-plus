use std::mem;

/// Our int keys are simply the big-endian representation bytes for unsigned ints,
/// but "sign-flipped" (xored msb) big-endian bytes for signed ints.
///
/// So that the representation of signed integers is in the right lexicographical order.
// TODO: Rename to `IntKey` after deprecating current `IntKey` (https://github.com/CosmWasm/cw-plus/issues/570)
pub trait CwIntKey: Sized + Copy {
    type Buf: AsRef<[u8]> + AsMut<[u8]> + Into<Vec<u8>> + Default;

    fn to_cw_bytes(&self) -> Self::Buf;
    fn from_cw_bytes(bytes: Self::Buf) -> Self;
}

macro_rules! cw_uint_keys {
    (for $($t:ty),+) => {
        $(impl CwIntKey for $t {
            type Buf = [u8; mem::size_of::<$t>()];

            #[inline]
            fn to_cw_bytes(&self) -> Self::Buf {
                self.to_be_bytes()
            }

            #[inline]
            fn from_cw_bytes(bytes: Self::Buf) -> Self {
                Self::from_be_bytes(bytes)
            }
        })*
    }
}

cw_uint_keys!(for u8, u16, u32, u64, u128);

macro_rules! cw_int_keys {
    (for $($t:ty, $ut:ty),+) => {
        $(impl CwIntKey for $t {
            type Buf = [u8; mem::size_of::<$t>()];

            #[inline]
            fn to_cw_bytes(&self) -> Self::Buf {
                (*self as $ut ^ <$t>::MIN as $ut).to_be_bytes()
            }

            #[inline]
            fn from_cw_bytes(bytes: Self::Buf) -> Self {
                (Self::from_be_bytes(bytes) as $ut ^ <$t>::MIN as $ut) as _
            }
        })*
    }
}

cw_int_keys!(for i8, u8, i16, u16, i32, u32, i64, u64, i128, u128);

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn x8_int_key_works() {
        assert_eq!(0x42u8.to_cw_bytes(), [0x42]);
        assert_eq!(0x42i8.to_cw_bytes(), [0xc2]);
        assert_eq!((-0x3ei8).to_cw_bytes(), [0x42]);
    }

    #[test]
    fn x16_int_key_works() {
        assert_eq!(0x4243u16.to_cw_bytes(), [0x42, 0x43]);
        assert_eq!(0x4243i16.to_cw_bytes(), [0xc2, 0x43]);
        assert_eq!((-0x3dbdi16).to_cw_bytes(), [0x42, 0x43]);
    }

    #[test]
    fn x32_int_key_works() {
        assert_eq!(0x424344u32.to_cw_bytes(), [0x00, 0x42, 0x43, 0x44]);
        assert_eq!(0x424344i32.to_cw_bytes(), [0x80, 0x42, 0x43, 0x44]);
        assert_eq!((-0x7fbdbcbci32).to_cw_bytes(), [0x00, 0x42, 0x43, 0x44]);
    }

    #[test]
    fn x64_int_key_works() {
        assert_eq!(
            0x42434445u64.to_cw_bytes(),
            [0x00, 0x00, 0x00, 0x00, 0x42, 0x43, 0x44, 0x45]
        );
        assert_eq!(
            0x42434445i64.to_cw_bytes(),
            [0x80, 0x00, 0x00, 0x00, 0x42, 0x43, 0x44, 0x45]
        );
        assert_eq!(
            (-0x7fffffffbdbcbbbbi64).to_cw_bytes(),
            [0x00, 0x00, 0x00, 0x00, 0x42, 0x43, 0x44, 0x45]
        );
    }

    #[test]
    fn x128_int_key_works() {
        assert_eq!(
            0x4243444546u128.to_cw_bytes(),
            [
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x42, 0x43, 0x44,
                0x45, 0x46
            ]
        );
        assert_eq!(
            0x4243444546i128.to_cw_bytes(),
            [
                0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x42, 0x43, 0x44,
                0x45, 0x46
            ]
        );
        assert_eq!(
            (-0x7fffffffffffffffffffffbdbcbbbabai128).to_cw_bytes(),
            [
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x42, 0x43, 0x44,
                0x45, 0x46
            ]
        );
    }

    #[test]
    fn unsigned_int_key_order() {
        assert!(0u32.to_cw_bytes() < 652u32.to_cw_bytes());
    }

    #[test]
    fn signed_int_key_order() {
        assert!((-321i32).to_cw_bytes() < 0i32.to_cw_bytes());
        assert!(0i32.to_cw_bytes() < 652i32.to_cw_bytes());
    }
}
