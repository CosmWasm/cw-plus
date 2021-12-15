use std::mem;

/// Our int keys are simply the Big-endian representation bytes for unsigned ints,
/// but "sign-flipped" (xored msb) Big-endian bytes for signed ints.
/// So that the representation of signed integers is correctly ordered lexicographically.
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
    (for $($t:ty),+) => {
        $(impl CwIntKey for $t {
            type Buf = [u8; mem::size_of::<$t>()];

            fn to_cw_bytes(&self) -> Self::Buf {
                let mut bytes = self.to_be_bytes();
                bytes[0] ^= 0x80;
                bytes
            }

            fn from_cw_bytes(bytes: Self::Buf) -> Self {
                let mut bytes = bytes;
                bytes[0] ^= 0x80;
                Self::from_be_bytes(bytes)
            }
        })*
    }
}

cw_int_keys!(for i8, i16, i32, i64, i128);
