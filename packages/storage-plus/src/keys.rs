use cosmwasm_std::Addr;
use std::marker::PhantomData;

use crate::addr::AddrRef;
use crate::helpers::namespaces_with_key;
use crate::Endian;

// pub trait PrimaryKey<'a>: Copy {
pub trait PrimaryKey<'a>: Clone {
    type Prefix: Prefixer<'a>;
    type SubPrefix: Prefixer<'a>;

    /// returns a slice of key steps, which can be optionally combined
    fn key(&self) -> Vec<&[u8]>;

    fn joined_key(&self) -> Vec<u8> {
        let keys = self.key();
        let l = keys.len();
        namespaces_with_key(&keys[0..l - 1], &keys[l - 1])
    }
}

impl<'a> PrimaryKey<'a> for &'a [u8] {
    type Prefix = ();
    type SubPrefix = ();

    fn key(&self) -> Vec<&[u8]> {
        // this is simple, we don't add more prefixes
        vec![self]
    }
}

// Provide a string version of this to raw encode strings
impl<'a> PrimaryKey<'a> for &'a str {
    type Prefix = ();
    type SubPrefix = ();

    fn key(&self) -> Vec<&[u8]> {
        // this is simple, we don't add more prefixes
        vec![self.as_bytes()]
    }
}

// use generics for combining there - so we can use &[u8], PkOwned, or IntKey
impl<'a, T: PrimaryKey<'a> + Prefixer<'a>, U: PrimaryKey<'a>> PrimaryKey<'a> for (T, U) {
    type Prefix = T;
    type SubPrefix = ();

    fn key(&self) -> Vec<&[u8]> {
        let mut keys = self.0.key();
        keys.extend(&self.1.key());
        keys
    }
}

// use generics for combining there - so we can use &[u8], PkOwned, or IntKey
impl<'a, T: PrimaryKey<'a> + Prefixer<'a>, U: PrimaryKey<'a> + Prefixer<'a>, V: PrimaryKey<'a>>
    PrimaryKey<'a> for (T, U, V)
{
    type Prefix = (T, U);
    type SubPrefix = T;

    fn key(&self) -> Vec<&[u8]> {
        let mut keys = self.0.key();
        keys.extend(&self.1.key());
        keys.extend(&self.2.key());
        keys
    }
}

// pub trait Prefixer<'a>: Copy {
pub trait Prefixer<'a> {
    /// returns 0 or more namespaces that should length-prefixed and concatenated for range searches
    fn prefix(&self) -> Vec<&[u8]>;
}

impl<'a> Prefixer<'a> for () {
    fn prefix(&self) -> Vec<&[u8]> {
        vec![]
    }
}

impl<'a> Prefixer<'a> for &'a [u8] {
    fn prefix(&self) -> Vec<&[u8]> {
        vec![self]
    }
}

impl<'a, T: Prefixer<'a>, U: Prefixer<'a>> Prefixer<'a> for (T, U) {
    fn prefix(&self) -> Vec<&[u8]> {
        let mut res = self.0.prefix();
        res.extend(self.1.prefix().into_iter());
        res
    }
}

impl<'a, T: Prefixer<'a>, U: Prefixer<'a>, V: Prefixer<'a>> Prefixer<'a> for (T, U, V) {
    fn prefix(&self) -> Vec<&[u8]> {
        let mut res = self.0.prefix();
        res.extend(self.1.prefix().into_iter());
        res.extend(self.2.prefix().into_iter());
        res
    }
}

// Provide a string version of this to raw encode strings
impl<'a> Prefixer<'a> for &'a str {
    fn prefix(&self) -> Vec<&[u8]> {
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
    type SubPrefix = ();

    fn key(&self) -> Vec<&[u8]> {
        vec![&self.0]
    }
}

impl<'a> Prefixer<'a> for PkOwned {
    fn prefix(&self) -> Vec<&[u8]> {
        vec![&self.0]
    }
}

/// type safe version to ensure address was validated before use.
impl<'a> PrimaryKey<'a> for &'a Addr {
    type Prefix = ();
    type SubPrefix = ();

    fn key(&self) -> Vec<&[u8]> {
        // this is simple, we don't add more prefixes
        vec![self.as_ref().as_bytes()]
    }
}

/// type safe version to ensure address was validated before use.
/// This is equivalent to &Addr but compatible with these lifetimes
impl<'a> PrimaryKey<'a> for AddrRef<'a> {
    type Prefix = ();
    type SubPrefix = ();

    fn key(&self) -> Vec<&[u8]> {
        // this is simple, we don't add more prefixes
        vec![self.as_bytes()]
    }
}

/// A type-safe way to use verified addresses as keys
impl<'a> Prefixer<'a> for AddrRef<'a> {
    fn prefix(&self) -> Vec<&[u8]> {
        vec![self.as_bytes()]
    }
}

// this auto-implements PrimaryKey for all the IntKey types (and more!)
impl<'a, T: AsRef<PkOwned> + From<PkOwned> + Clone> PrimaryKey<'a> for T {
    type Prefix = ();
    type SubPrefix = ();

    fn key(&self) -> Vec<&[u8]> {
        self.as_ref().key()
    }
}

// this auto-implements Prefixer for all the IntKey types (and more!)
impl<'a, T: AsRef<PkOwned>> Prefixer<'a> for T {
    fn prefix(&self) -> Vec<&[u8]> {
        self.as_ref().prefix()
    }
}

pub type U8Key = IntKey<u8>;
pub type U16Key = IntKey<u16>;
pub type U32Key = IntKey<u32>;
pub type U64Key = IntKey<u64>;
pub type U128Key = IntKey<u128>;

pub type I8Key = IntKey<i8>;
pub type I16Key = IntKey<i16>;
pub type I32Key = IntKey<i32>;
pub type I64Key = IntKey<i64>;
pub type I128Key = IntKey<i128>;

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
        // TODO: assert proper length
        IntKey {
            wrapped: wrap,
            data: PhantomData,
        }
    }
}

impl<T: Endian> From<Vec<u8>> for IntKey<T> {
    fn from(wrap: Vec<u8>) -> Self {
        PkOwned(wrap).into()
    }
}

impl<T: Endian> From<IntKey<T>> for Vec<u8> {
    fn from(k: IntKey<T>) -> Vec<u8> {
        k.wrapped.0
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
        type K<'a> = &'a str;

        let k: K = "hello";
        let path = k.key();
        assert_eq!(1, path.len());
        assert_eq!("hello".as_bytes(), path[0]);

        let joined = k.joined_key();
        assert_eq!(joined, b"hello")
    }

    #[test]
    fn nested_str_key_works() {
        type K<'a> = (&'a str, &'a [u8]);

        let k: K = ("hello", b"world");
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
    fn proper_prefixes() {
        let simple: &str = "hello";
        assert_eq!(simple.prefix(), vec![b"hello"]);

        let pair: (U32Key, &[u8]) = (12345.into(), b"random");
        let one: Vec<u8> = vec![0, 0, 48, 57];
        let two: Vec<u8> = b"random".to_vec();
        assert_eq!(pair.prefix(), vec![one.as_slice(), two.as_slice()]);

        let triple: (&str, U32Key, &[u8]) = ("begin", 12345.into(), b"end");
        let one: Vec<u8> = b"begin".to_vec();
        let two: Vec<u8> = vec![0, 0, 48, 57];
        let three: Vec<u8> = b"end".to_vec();
        assert_eq!(
            triple.prefix(),
            vec![one.as_slice(), two.as_slice(), three.as_slice()]
        );
    }
}
