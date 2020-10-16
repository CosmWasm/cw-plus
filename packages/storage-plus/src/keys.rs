use std::marker::PhantomData;
use std::str::from_utf8;

use crate::helpers::{decode_length, namespaces_with_key};
use crate::Endian;

// pub trait PrimaryKey<'a>: Copy {
pub trait PrimaryKey<'a>: Clone {
    type Prefix: Prefixer<'a>;

    /// returns a slice of key steps, which can be optionally combined
    fn key<'b>(&'b self) -> Vec<&'b [u8]>;

    fn joined_key(&self) -> Vec<u8> {
        let keys = self.key();
        let l = keys.len();
        namespaces_with_key(&keys[0..l - 1], &keys[l - 1])
    }

    /// extracts a single or composite key from a joined key,
    /// only lives as long as the original bytes
    fn parse_key(serialized: &'a [u8]) -> Self;
}

// optional type aliases to refer to them easier
type Pk0 = ();
type Pk1<'a> = &'a [u8];
type Pk2<'a, T = &'a [u8], U = &'a [u8]> = (T, U);

type PkStr<'a> = &'a str;

impl<'a> PrimaryKey<'a> for Pk1<'a> {
    type Prefix = Pk0;

    fn key<'b>(&'b self) -> Vec<&'b [u8]> {
        // this is simple, we don't add more prefixes
        vec![self]
    }

    fn parse_key(serialized: &'a [u8]) -> Self {
        serialized
    }
}

// Provide a string version of this to raw encode strings
impl<'a> PrimaryKey<'a> for PkStr<'a> {
    type Prefix = Pk0;

    fn key<'b>(&'b self) -> Vec<&'b [u8]> {
        // this is simple, we don't add more prefixes
        vec![self.as_bytes()]
    }

    fn parse_key(serialized: &'a [u8]) -> Self {
        from_utf8(serialized).unwrap()
    }
}

// use generics for combining there - so we can use &[u8], PkOwned, or IntKey
impl<'a, T: PrimaryKey<'a> + Prefixer<'a>, U: PrimaryKey<'a>> PrimaryKey<'a> for (T, U) {
    type Prefix = T;

    fn key<'b>(&'b self) -> Vec<&'b [u8]> {
        let mut keys = self.0.key();
        keys.extend(&self.1.key());
        keys
    }

    fn parse_key(serialized: &'a [u8]) -> Self {
        let l = decode_length(&serialized[0..2]);
        let first = &serialized[2..l + 2];
        let second = &serialized[l + 2..];
        (T::parse_key(first), U::parse_key(second))
    }
}

// pub trait Prefixer<'a>: Copy {
pub trait Prefixer<'a> {
    /// returns 0 or more namespaces that should length-prefixed and concatenated for range searches
    fn prefix<'b>(&'b self) -> Vec<&'b [u8]>;
}

impl<'a> Prefixer<'a> for Pk0 {
    fn prefix<'b>(&'b self) -> Vec<&'b [u8]> {
        vec![]
    }
}

impl<'a> Prefixer<'a> for Pk1<'a> {
    fn prefix<'b>(&'b self) -> Vec<&'b [u8]> {
        vec![self]
    }
}

impl<'a> Prefixer<'a> for Pk2<'a> {
    fn prefix<'b>(&'b self) -> Vec<&'b [u8]> {
        vec![self.0, self.1]
    }
}

// Provide a string version of this to raw encode strings
impl<'a> Prefixer<'a> for PkStr<'a> {
    fn prefix<'b>(&'b self) -> Vec<&'b [u8]> {
        vec![self.as_bytes()]
    }
}

// this is a marker for the Map.range() helper, so we can detect () in Generic bounds
pub trait EmptyPrefix {
    fn new() -> Self;
}

impl EmptyPrefix for () {
    fn new() {}
}

// Add support for an dynamic keys - constructor functions below
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PkOwned(pub Vec<u8>);

impl<'a> PrimaryKey<'a> for PkOwned {
    type Prefix = ();

    fn key<'b>(&'b self) -> Vec<&'b [u8]> {
        vec![&self.0]
    }

    fn parse_key(serialized: &'a [u8]) -> Self {
        PkOwned(serialized.to_vec())
    }
}

impl<'a> Prefixer<'a> for PkOwned {
    fn prefix<'b>(&'b self) -> Vec<&'b [u8]> {
        vec![&self.0]
    }
}

// this auto-implements PrimaryKey for all the IntKey types (and more!)
impl<'a, T: AsRef<PkOwned> + From<PkOwned> + Clone> PrimaryKey<'a> for T {
    type Prefix = ();

    fn key<'b>(&'b self) -> Vec<&'b [u8]> {
        self.as_ref().key()
    }

    fn parse_key(serialized: &'a [u8]) -> Self {
        PkOwned::parse_key(serialized).into()
    }
}

// this auto-implements Prefixer for all the IntKey types (and more!)
impl<'a, T: AsRef<PkOwned>> Prefixer<'a> for T {
    fn prefix<'b>(&'b self) -> Vec<&'b [u8]> {
        self.as_ref().prefix()
    }
}

pub type U16Key = IntKey<u16>;
pub type U32Key = IntKey<u32>;
pub type U64Key = IntKey<u64>;
pub type U128Key = IntKey<u128>;

/// It will cast one-particular int type into a Key via PkOwned, ensuring you don't mix up u32 and u64
/// You can use new or the from/into pair to build a key from an int:
///
///   let k = U64Key::new(12345);
///   let k = U32Key::from(12345);
///   let k: U16Key = 12345.into();
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IntKey<T: Endian> {
    pub wrapped: PkOwned,
    pub data: PhantomData<T>,
}

impl<T: Endian> IntKey<T> {
    pub fn new(val: T) -> Self {
        IntKey {
            wrapped: PkOwned(val.to_be_bytes().into()),
            data: PhantomData,
        }
    }
}

impl<T: Endian> From<T> for IntKey<T> {
    fn from(val: T) -> Self {
        IntKey::new(val)
    }
}

impl<T: Endian> From<PkOwned> for IntKey<T> {
    fn from(wrap: PkOwned) -> Self {
        IntKey {
            wrapped: wrap,
            data: PhantomData,
        }
    }
}

impl<T: Endian> AsRef<PkOwned> for IntKey<T> {
    fn as_ref(&self) -> &PkOwned {
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

    #[test]
    fn str_key_works() {
        let k: &str = "hello";
        let path = k.key();
        assert_eq!(1, path.len());
        assert_eq!("hello".as_bytes(), path[0]);

        let joined = k.joined_key();
        let parsed = PkStr::parse_key(&joined);
        assert_eq!(parsed, "hello");
    }

    #[test]
    fn nested_str_key_works() {
        let k: (&str, &[u8]) = ("hello", b"world");
        let path = k.key();
        assert_eq!(2, path.len());
        assert_eq!("hello".as_bytes(), path[0]);
        assert_eq!("world".as_bytes(), path[1]);
    }

    #[test]
    fn composite_byte_key() {
        let k: (&[u8], &[u8]) = (b"foo", b"bar");
        let path = k.key();
        assert_eq!(2, path.len());
        assert_eq!(path, vec![b"foo", b"bar"]);
    }

    #[test]
    fn composite_int_key() {
        // Note we don't spec the int types (u32, u64) on the right,
        // just the keys they convert into
        let k: (U32Key, U64Key) = (123.into(), 87654.into());
        let path = k.key();
        assert_eq!(2, path.len());
        assert_eq!(4, path[0].len());
        assert_eq!(8, path[1].len());
        assert_eq!(path[0].to_vec(), 123u32.to_be_bytes().to_vec());
        assert_eq!(path[1].to_vec(), 87654u64.to_be_bytes().to_vec());
    }

    #[test]
    fn nested_composite_keys() {
        // use this to ensure proper type-casts below
        let foo: &[u8] = b"foo";
        // this function tests how well the generics extend to "edge cases"
        let k: ((&[u8], &[u8]), &[u8]) = ((foo, b"bar"), b"zoom");
        let path = k.key();
        assert_eq!(3, path.len());
        assert_eq!(path, vec![foo, b"bar", b"zoom"]);

        // ensure prefix also works
        let dir = k.0.prefix();
        assert_eq!(2, dir.len());
        assert_eq!(dir, vec![foo, b"bar"]);
    }

    #[test]
    fn parse_joined_keys_pk1() {
        let key: Pk1 = b"four";
        let joined = key.joined_key();
        assert_eq!(key, joined.as_slice());
        let parsed = Pk1::parse_key(&joined);
        assert_eq!(key, parsed);
    }

    #[test]
    fn parse_joined_keys_pk2() {
        let key: Pk2 = (b"four", b"square");
        let joined = key.joined_key();
        assert_eq!(4 + 6 + 2, joined.len());
        let parsed = Pk2::parse_key(&joined);
        assert_eq!(key, parsed);
    }

    #[test]
    fn parse_joined_keys_int() {
        let key: U64Key = 12345678.into();
        let joined = key.joined_key();
        assert_eq!(8, joined.len());
        let parsed = U64Key::parse_key(&joined);
        assert_eq!(key, parsed);
    }

    #[test]
    fn parse_joined_keys_string_int() {
        let key: (U32Key, &str) = (54321.into(), "random");
        let joined = key.joined_key();
        assert_eq!(2 + 4 + 6, joined.len());
        let parsed = <(U32Key, &str)>::parse_key(&joined);
        assert_eq!(key, parsed);
        assert_eq!("random", parsed.1);
    }
}
