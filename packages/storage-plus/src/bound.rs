#![cfg(feature = "iterator")]

use cosmwasm_std::Addr;
use std::marker::PhantomData;

use crate::de::KeyDeserialize;
use crate::{Prefixer, PrimaryKey};

/// `RawBound` is used to define the two ends of a range, more explicit than `Option<u8>`.
/// `None` means that we don't limit that side of the range at all.
/// `Inclusive` means we use the given bytes as a limit and *include* anything at that exact key.
/// `Exclusive` means we use the given bytes as a limit and *exclude* anything at that exact key.
/// See `Bound` for a type safe way to build these bounds.
#[derive(Clone, Debug)]
pub enum RawBound {
    Inclusive(Vec<u8>),
    Exclusive(Vec<u8>),
}

/// `Bound` is used to define the two ends of a range.
/// `None` means that we don't limit that side of the range at all.
/// `Inclusive` means we use the given value as a limit and *include* anything at that exact key.
/// `Exclusive` means we use the given value as a limit and *exclude* anything at that exact key.
#[derive(Clone, Debug)]
pub enum Bound<'a, K: PrimaryKey<'a>> {
    Inclusive((K, PhantomData<&'a bool>)),
    Exclusive((K, PhantomData<&'a bool>)),
    InclusiveRaw(Vec<u8>),
    ExclusiveRaw(Vec<u8>),
}

impl<'a, K: PrimaryKey<'a>> Bound<'a, K> {
    pub fn inclusive<T: Into<K>>(k: T) -> Self {
        Self::Inclusive((k.into(), PhantomData))
    }

    pub fn exclusive<T: Into<K>>(k: T) -> Self {
        Self::Exclusive((k.into(), PhantomData))
    }

    pub fn to_raw_bound(&self) -> RawBound {
        match self {
            Bound::Inclusive((k, _)) => RawBound::Inclusive(k.joined_key()),
            Bound::Exclusive((k, _)) => RawBound::Exclusive(k.joined_key()),
            Bound::ExclusiveRaw(raw_k) => RawBound::Exclusive(raw_k.clone()),
            Bound::InclusiveRaw(raw_k) => RawBound::Inclusive(raw_k.clone()),
        }
    }
}

#[derive(Clone, Debug)]
pub enum PrefixBound<'a, K: Prefixer<'a>> {
    Inclusive((K, PhantomData<&'a bool>)),
    Exclusive((K, PhantomData<&'a bool>)),
}

impl<'a, K: Prefixer<'a>> PrefixBound<'a, K> {
    pub fn inclusive<T: Into<K>>(k: T) -> Self {
        Self::Inclusive((k.into(), PhantomData))
    }

    pub fn exclusive<T: Into<K>>(k: T) -> Self {
        Self::Exclusive((k.into(), PhantomData))
    }

    pub fn to_raw_bound(&self) -> RawBound {
        match self {
            PrefixBound::Exclusive((k, _)) => RawBound::Exclusive(k.joined_prefix()),
            PrefixBound::Inclusive((k, _)) => RawBound::Inclusive(k.joined_prefix()),
        }
    }
}

pub trait Bounder<'a>: PrimaryKey<'a> + Sized {
    fn inclusive_bound(self) -> Option<Bound<'a, Self>>;
    fn exclusive_bound(self) -> Option<Bound<'a, Self>>;
}

impl<'a> Bounder<'a> for () {
    fn inclusive_bound(self) -> Option<Bound<'a, Self>> {
        None
    }
    fn exclusive_bound(self) -> Option<Bound<'a, Self>> {
        None
    }
}

impl<'a> Bounder<'a> for &'a [u8] {
    fn inclusive_bound(self) -> Option<Bound<'a, Self>> {
        Some(Bound::inclusive(self))
    }
    fn exclusive_bound(self) -> Option<Bound<'a, Self>> {
        Some(Bound::exclusive(self))
    }
}

impl<
        'a,
        T: PrimaryKey<'a> + KeyDeserialize + Prefixer<'a> + Clone,
        U: PrimaryKey<'a> + KeyDeserialize + Clone,
    > Bounder<'a> for (T, U)
{
    fn inclusive_bound(self) -> Option<Bound<'a, Self>> {
        Some(Bound::inclusive(self))
    }
    fn exclusive_bound(self) -> Option<Bound<'a, Self>> {
        Some(Bound::exclusive(self))
    }
}

impl<
        'a,
        T: PrimaryKey<'a> + Prefixer<'a> + Clone,
        U: PrimaryKey<'a> + Prefixer<'a> + KeyDeserialize + Clone,
        V: PrimaryKey<'a> + KeyDeserialize + Clone,
    > Bounder<'a> for (T, U, V)
{
    fn inclusive_bound(self) -> Option<Bound<'a, Self>> {
        Some(Bound::inclusive(self))
    }
    fn exclusive_bound(self) -> Option<Bound<'a, Self>> {
        Some(Bound::exclusive(self))
    }
}

impl<'a> Bounder<'a> for &'a str {
    fn inclusive_bound(self) -> Option<Bound<'a, Self>> {
        Some(Bound::inclusive(self))
    }
    fn exclusive_bound(self) -> Option<Bound<'a, Self>> {
        Some(Bound::exclusive(self))
    }
}

impl<'a> Bounder<'a> for String {
    fn inclusive_bound(self) -> Option<Bound<'a, Self>> {
        Some(Bound::inclusive(self))
    }
    fn exclusive_bound(self) -> Option<Bound<'a, Self>> {
        Some(Bound::exclusive(self))
    }
}

impl<'a> Bounder<'a> for Vec<u8> {
    fn inclusive_bound(self) -> Option<Bound<'a, Self>> {
        Some(Bound::inclusive(self))
    }
    fn exclusive_bound(self) -> Option<Bound<'a, Self>> {
        Some(Bound::exclusive(self))
    }
}

impl<'a> Bounder<'a> for &'a Addr {
    fn inclusive_bound(self) -> Option<Bound<'a, Self>> {
        Some(Bound::inclusive(self))
    }
    fn exclusive_bound(self) -> Option<Bound<'a, Self>> {
        Some(Bound::exclusive(self))
    }
}

impl<'a> Bounder<'a> for Addr {
    fn inclusive_bound(self) -> Option<Bound<'a, Self>> {
        Some(Bound::inclusive(self))
    }
    fn exclusive_bound(self) -> Option<Bound<'a, Self>> {
        Some(Bound::exclusive(self))
    }
}

macro_rules! integer_bound {
    (for $($t:ty),+) => {
        $(impl<'a> Bounder<'a> for $t {
            fn inclusive_bound(self) -> Option<Bound<'a, Self>> {
                Some(Bound::inclusive(self))
            }
            fn exclusive_bound(self) -> Option<Bound<'a, Self>> {
                Some(Bound::exclusive(self))
            }
        })*
    }
}

integer_bound!(for i8, u8, i16, u16, i32, u32, i64, u64);
