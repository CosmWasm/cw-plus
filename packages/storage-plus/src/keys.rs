use crate::Endian;
use std::marker::PhantomData;

pub trait PrimaryKey<'a> {
    type Prefix: Prefixer<'a>;

    /// returns a slice of key steps, which can be optionally combined
    fn key<'b>(&'b self) -> Vec<&'b [u8]>;
}

impl<'a> PrimaryKey<'a> for &'a [u8] {
    type Prefix = ();

    fn key<'b>(&'b self) -> Vec<&'b [u8]> {
        // this is simple, we don't add more prefixes
        vec![self]
    }
}

impl<'a> PrimaryKey<'a> for (&'a [u8], &'a [u8]) {
    type Prefix = &'a [u8];

    fn key<'b>(&'b self) -> Vec<&'b [u8]> {
        vec![self.0, self.1]
    }
}

impl<'a> PrimaryKey<'a> for (&'a [u8], &'a [u8], &'a [u8]) {
    type Prefix = (&'a [u8], &'a [u8]);

    fn key<'b>(&'b self) -> Vec<&'b [u8]> {
        vec![self.0, self.1, self.2]
    }
}

pub trait Prefixer<'a> {
    /// returns 0 or more namespaces that should length-prefixed and concatenated for range searches
    fn prefix(&self) -> Vec<&'a [u8]>;
}

impl<'a> Prefixer<'a> for () {
    fn prefix(&self) -> Vec<&'a [u8]> {
        vec![]
    }
}

impl<'a> Prefixer<'a> for &'a [u8] {
    fn prefix(&self) -> Vec<&'a [u8]> {
        vec![self]
    }
}

impl<'a> Prefixer<'a> for (&'a [u8], &'a [u8]) {
    fn prefix(&self) -> Vec<&'a [u8]> {
        vec![self.0, self.1]
    }
}

// Add support for an dynamic keys - constructor functions below
pub struct Pk1Owned(pub Vec<u8>);

impl<'a> PrimaryKey<'a> for Pk1Owned {
    type Prefix = ();

    fn key<'b>(&'b self) -> Vec<&'b [u8]> {
        vec![&self.0]
    }
}

// this auto-implements PrimaryKey for all the IntKey types (and more!)
impl<'a, T: AsRef<Pk1Owned>> PrimaryKey<'a> for T {
    type Prefix = ();

    fn key<'b>(&'b self) -> Vec<&'b [u8]> {
        self.as_ref().key()
    }
}

pub type U16Key = IntKey<u16>;
pub type U32Key = IntKey<u32>;
pub type U64Key = IntKey<u64>;
pub type U128Key = IntKey<u128>;

// this reuses Pk1Owned logic with a particular type
pub struct IntKey<T: Endian> {
    pub wrapped: Pk1Owned,
    pub data: PhantomData<T>,
}

impl<T: Endian> IntKey<T> {
    pub fn new(val: T) -> Self {
        IntKey {
            wrapped: Pk1Owned(val.to_be_bytes().as_ref().to_vec()),
            data: PhantomData,
        }
    }
}

impl<T: Endian> From<T> for IntKey<T> {
    fn from(val: T) -> Self {
        IntKey::new(val)
    }
}

impl<T: Endian> AsRef<Pk1Owned> for IntKey<T> {
    fn as_ref(&self) -> &Pk1Owned {
        &self.wrapped
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn u64key_works() {
        let k: U64Key = 134u64.into();
        let path = k.key();
        assert_eq!(1, path.len());
        assert_eq!(134u64.to_be_bytes().to_vec(), path[0].to_vec());
    }

    #[test]
    fn u32key_works() {
        let k: U32Key = 4242u32.into();
        let path = k.key();
        assert_eq!(1, path.len());
        assert_eq!(4242u32.to_be_bytes().to_vec(), path[0].to_vec());
    }
}
