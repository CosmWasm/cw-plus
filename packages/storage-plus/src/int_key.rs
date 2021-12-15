use std::mem;

/// Our int keys are simply the big-endian representation bytes for unsigned ints,
/// but "sign-flipped" (xored msb) big-endian bytes for signed ints.
///
/// So that the representation of signed integers is in the right lexicographical order.
// TODO: Rename to `IntKey` when deprecating current `IntKey`
pub trait CwIntKey: Sized + Copy {
    type Buf: AsRef<[u8]> + AsMut<[u8]> + Into<Vec<u8>> + Default;

    fn to_cw_bytes(&self) -> Self::Buf;
    fn from_cw_bytes(bytes: Self::Buf) -> Self;
}

macro_rules! cw_uint_keys {
    (for $($t:ty),+) => {
        $(impl CwIntKey for $t {
            type Buf = [u8; mem::size_of::<$t>()];

            fn to_cw_bytes(&self) -> Self::Buf {
                self.to_be_bytes()
            }

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

            fn to_cw_bytes(&self) -> Self::Buf {
                (*self as $ut ^ <$t>::MIN as $ut).to_be_bytes()
            }

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
        let k: u8 = 42u8;
        assert_eq!(k.to_cw_bytes(), k.to_be_bytes());

        let k: i8 = 42i8;
        assert_eq!(k.to_cw_bytes(), (k as u8 ^ 0x80).to_be_bytes());

        let k: i8 = -42i8;
        assert_eq!(k.to_cw_bytes(), (k as u8 ^ 0x80).to_be_bytes());
    }

    #[test]
    fn x16_int_key_works() {
        let k: u16 = 4243u16;
        assert_eq!(k.to_cw_bytes(), k.to_be_bytes());

        let k: i16 = 4445i16;
        assert_eq!(k.to_cw_bytes(), (k as u16 ^ 0x8000).to_be_bytes());

        let k: i16 = -4748i16;
        assert_eq!(k.to_cw_bytes(), (k as u16 ^ 0x8000).to_be_bytes());
    }

    #[test]
    fn x32_int_key_works() {
        let k: u32 = 424344u32;
        assert_eq!(k.to_cw_bytes(), k.to_be_bytes());

        let k: i32 = 454647i32;
        assert_eq!(k.to_cw_bytes(), (k as u32 ^ 0x80000000).to_be_bytes());

        let k: i32 = -484950i32;
        assert_eq!(k.to_cw_bytes(), (k as u32 ^ 0x80000000).to_be_bytes());
    }

    #[test]
    fn x64_int_key_works() {
        let k: u64 = 42434445u64;
        assert_eq!(k.to_cw_bytes(), k.to_be_bytes());

        let k: i64 = 46474849i64;
        assert_eq!(
            k.to_cw_bytes(),
            (k as u64 ^ 0x8000000000000000).to_be_bytes()
        );

        let k: i64 = -50515253i64;
        assert_eq!(
            k.to_cw_bytes(),
            (k as u64 ^ 0x8000000000000000).to_be_bytes()
        );
    }

    #[test]
    fn x128_int_key_works() {
        let k: u128 = 4243444546u128;
        assert_eq!(k.to_cw_bytes(), k.to_be_bytes());

        let k: i128 = 4748495051i128;
        assert_eq!(
            k.to_cw_bytes(),
            (k as u128 ^ 0x80000000000000000000000000000000).to_be_bytes()
        );

        let k: i128 = -5253545556i128;
        assert_eq!(
            k.to_cw_bytes(),
            (k as u128 ^ 0x80000000000000000000000000000000).to_be_bytes()
        );
    }
}
