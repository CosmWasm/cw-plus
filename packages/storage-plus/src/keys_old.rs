use crate::de::KeyDeserialize;
use crate::keys::Key;
#[cfg(feature = "iterator")]
use crate::{Bound, Bounder};
use crate::{Endian, Prefixer, PrimaryKey};
use std::marker::PhantomData;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IntKeyOld<T: Endian> {
    pub wrapped: Vec<u8>,
    pub data: PhantomData<T>,
}

impl<T: Endian> IntKeyOld<T> {
    pub fn new(val: T) -> Self {
        IntKeyOld {
            wrapped: val.to_be_bytes().into(),
            data: PhantomData,
        }
    }
}

impl<T: Endian> From<T> for IntKeyOld<T> {
    fn from(val: T) -> Self {
        IntKeyOld::new(val)
    }
}

impl<T: Endian> From<Vec<u8>> for IntKeyOld<T> {
    fn from(wrap: Vec<u8>) -> Self {
        IntKeyOld {
            wrapped: wrap,
            data: PhantomData,
        }
    }
}

impl<T: Endian> From<IntKeyOld<T>> for Vec<u8> {
    fn from(k: IntKeyOld<T>) -> Vec<u8> {
        k.wrapped
    }
}

// this auto-implements PrimaryKey for all the IntKeyOld types
impl<'a, T: Endian + Clone> PrimaryKey<'a> for IntKeyOld<T>
where
    IntKeyOld<T>: KeyDeserialize,
{
    type Prefix = ();
    type SubPrefix = ();
    type Suffix = Self;
    type SuperSuffix = Self;

    fn key(&self) -> Vec<Key> {
        self.wrapped.key()
    }
}

// this auto-implements Prefixer for all the IntKey types
impl<'a, T: Endian> Prefixer<'a> for IntKeyOld<T> {
    fn prefix(&self) -> Vec<Key> {
        self.wrapped.prefix()
    }
}

// this auto-implements Bounder for all the IntKey types
#[cfg(feature = "iterator")]
impl<'a, T: Endian> Bounder<'a> for IntKeyOld<T>
where
    IntKeyOld<T>: KeyDeserialize,
{
    fn inclusive_bound(self) -> Option<Bound<'a, Self>> {
        Some(Bound::inclusive(self))
    }

    fn exclusive_bound(self) -> Option<Bound<'a, Self>> {
        Some(Bound::exclusive(self))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn u64key_old_works() {
        let k: IntKeyOld<u64> = 134u64.into();
        let path = k.key();
        assert_eq!(1, path.len());
        assert_eq!(134u64.to_be_bytes(), path[0].as_ref());
    }

    #[test]
    fn i32key_old_works() {
        let k: IntKeyOld<i32> = 4242i32.into();
        let path = k.key();
        assert_eq!(1, path.len());
        assert_eq!(4242i32.to_be_bytes(), path[0].as_ref());

        let k: IntKeyOld<i32> = IntKeyOld::<i32>::from(-4242i32);
        let path = k.key();
        assert_eq!(1, path.len());
        assert_eq!((-4242i32).to_be_bytes(), path[0].as_ref());
    }
}
