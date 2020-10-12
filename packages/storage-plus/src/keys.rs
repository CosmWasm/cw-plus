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

// use generics for combining there - so we can use &[u8], PkOwned, or IntKey
impl<'a, T: PrimaryKey<'a> + Prefixer<'a>, U: PrimaryKey<'a>> PrimaryKey<'a> for (T, U) {
    type Prefix = T;

    fn key<'b>(&'b self) -> Vec<&'b [u8]> {
        let mut keys = self.0.key();
        keys.extend(&self.1.key());
        keys
    }
}

// Future work: add more types - 3 or more or slices?
// Right now 3 could be done via ((a, b), c)

pub trait Prefixer<'a> {
    /// returns 0 or more namespaces that should length-prefixed and concatenated for range searches
    fn prefix<'b>(&'b self) -> Vec<&'b [u8]>;
}

impl<'a> Prefixer<'a> for () {
    fn prefix<'b>(&'b self) -> Vec<&'b [u8]> {
        vec![]
    }
}

impl<'a> Prefixer<'a> for &'a [u8] {
    fn prefix<'b>(&'b self) -> Vec<&'b [u8]> {
        vec![self]
    }
}

impl<'a> Prefixer<'a> for (&'a [u8], &'a [u8]) {
    fn prefix<'b>(&'b self) -> Vec<&'b [u8]> {
        vec![self.0, self.1]
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
pub struct PkOwned(pub Vec<u8>);

impl<'a> PrimaryKey<'a> for PkOwned {
    type Prefix = ();

    fn key<'b>(&'b self) -> Vec<&'b [u8]> {
        vec![&self.0]
    }
}

impl<'a> Prefixer<'a> for PkOwned {
    fn prefix<'b>(&'b self) -> Vec<&'b [u8]> {
        vec![&self.0]
    }
}

// this auto-implements PrimaryKey for all the IntKey types (and more!)
impl<'a, T: AsRef<PkOwned>> PrimaryKey<'a> for T {
    type Prefix = ();

    fn key<'b>(&'b self) -> Vec<&'b [u8]> {
        self.as_ref().key()
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
}
