use serde::de::DeserializeOwned;
use serde::Serialize;
use std::marker::PhantomData;

#[cfg(feature = "iterator")]
use crate::de::KeyDeserialize;
use crate::helpers::query_raw;
#[cfg(feature = "iterator")]
use crate::iter_helpers::{deserialize_kv, deserialize_v};
#[cfg(feature = "iterator")]
use crate::keys::Prefixer;
use crate::keys::PrimaryKey;
use crate::path::Path;
#[cfg(feature = "iterator")]
use crate::prefix::{namespaced_prefix_range, Bound, Prefix, PrefixBound};
use cosmwasm_std::{from_slice, Addr, QuerierWrapper, StdError, StdResult, Storage};

#[derive(Debug, Clone)]
pub struct Map<'a, K, T> {
    namespace: &'a [u8],
    // see https://doc.rust-lang.org/std/marker/struct.PhantomData.html#unused-type-parameters for why this is needed
    key_type: PhantomData<K>,
    data_type: PhantomData<T>,
}

impl<'a, K, T> Map<'a, K, T> {
    pub const fn new(namespace: &'a str) -> Self {
        Map {
            namespace: namespace.as_bytes(),
            data_type: PhantomData,
            key_type: PhantomData,
        }
    }
}

impl<'a, K, T> Map<'a, K, T>
where
    T: Serialize + DeserializeOwned,
    K: PrimaryKey<'a>,
{
    pub fn key(&self, k: K) -> Path<T> {
        Path::new(self.namespace, &k.key())
    }

    #[cfg(feature = "iterator")]
    pub fn prefix(&self, p: K::Prefix) -> Prefix<Vec<u8>, T> {
        Prefix::new(self.namespace, &p.prefix())
    }

    #[cfg(feature = "iterator")]
    pub fn sub_prefix(&self, p: K::SubPrefix) -> Prefix<Vec<u8>, T> {
        Prefix::new(self.namespace, &p.prefix())
    }

    #[cfg(feature = "iterator")]
    pub(crate) fn no_prefix(&self) -> Prefix<Vec<u8>, T> {
        Prefix::new(self.namespace, &[])
    }

    pub fn save(&self, store: &mut dyn Storage, k: K, data: &T) -> StdResult<()> {
        self.key(k).save(store, data)
    }

    pub fn remove(&self, store: &mut dyn Storage, k: K) {
        self.key(k).remove(store)
    }

    /// load will return an error if no data is set at the given key, or on parse error
    pub fn load(&self, store: &dyn Storage, k: K) -> StdResult<T> {
        self.key(k).load(store)
    }

    /// may_load will parse the data stored at the key if present, returns Ok(None) if no data there.
    /// returns an error on issues parsing
    pub fn may_load(&self, store: &dyn Storage, k: K) -> StdResult<Option<T>> {
        self.key(k).may_load(store)
    }

    /// has returns true or false if any data is at this key, without parsing or interpreting the
    /// contents.
    pub fn has(&self, store: &dyn Storage, k: K) -> bool {
        self.key(k).has(store)
    }

    /// Loads the data, perform the specified action, and store the result
    /// in the database. This is shorthand for some common sequences, which may be useful.
    ///
    /// If the data exists, `action(Some(value))` is called. Otherwise `action(None)` is called.
    pub fn update<A, E>(&self, store: &mut dyn Storage, k: K, action: A) -> Result<T, E>
    where
        A: FnOnce(Option<T>) -> Result<T, E>,
        E: From<StdError>,
    {
        self.key(k).update(store, action)
    }

    /// If you import the proper Map from the remote contract, this will let you read the data
    /// from a remote contract in a type-safe way using WasmQuery::RawQuery
    pub fn query(
        &self,
        querier: &QuerierWrapper,
        remote_contract: Addr,
        k: K,
    ) -> StdResult<Option<T>> {
        let key = self.key(k).storage_key.into();
        let result = query_raw(querier, remote_contract, key)?;
        if result.is_empty() {
            Ok(None)
        } else {
            from_slice(&result).map(Some)
        }
    }
}

#[cfg(feature = "iterator")]
impl<'a, K, T> Map<'a, K, T>
where
    T: Serialize + DeserializeOwned,
    K: PrimaryKey<'a>,
{
    pub fn sub_prefix_de(&self, p: K::SubPrefix) -> Prefix<K::SuperSuffix, T> {
        Prefix::new(self.namespace, &p.prefix())
    }

    pub fn prefix_de(&self, p: K::Prefix) -> Prefix<K::Suffix, T> {
        Prefix::new(self.namespace, &p.prefix())
    }
}

// short-cut for simple keys, rather than .prefix(()).range(...)
#[cfg(feature = "iterator")]
impl<'a, K, T> Map<'a, K, T>
where
    T: Serialize + DeserializeOwned,
    // TODO: this should only be when K::Prefix == ()
    // Other cases need to call prefix() first
    K: PrimaryKey<'a>,
{
    /// while range assumes you set the prefix to one element and call range over the last one,
    /// prefix_range accepts bounds for the lowest and highest elements of the Prefix we wish to
    /// accept, and iterates over those. There are some issues that distinguish these to and blindly
    /// casting to Vec<u8> doesn't solve them.
    pub fn prefix_range<'c>(
        &self,
        store: &'c dyn Storage,
        min: Option<PrefixBound<'a, K::Prefix>>,
        max: Option<PrefixBound<'a, K::Prefix>>,
        order: cosmwasm_std::Order,
    ) -> Box<dyn Iterator<Item = StdResult<cosmwasm_std::Record<T>>> + 'c>
    where
        T: 'c,
        'a: 'c,
    {
        let mapped =
            namespaced_prefix_range(store, self.namespace, min, max, order).map(deserialize_v);
        Box::new(mapped)
    }

    pub fn range<'c>(
        &self,
        store: &'c dyn Storage,
        min: Option<Bound>,
        max: Option<Bound>,
        order: cosmwasm_std::Order,
    ) -> Box<dyn Iterator<Item = StdResult<cosmwasm_std::Record<T>>> + 'c>
    where
        T: 'c,
    {
        self.no_prefix().range(store, min, max, order)
    }

    pub fn keys<'c>(
        &self,
        store: &'c dyn Storage,
        min: Option<Bound>,
        max: Option<Bound>,
        order: cosmwasm_std::Order,
    ) -> Box<dyn Iterator<Item = Vec<u8>> + 'c>
    where
        T: 'c,
    {
        self.no_prefix().keys(store, min, max, order)
    }
}

#[cfg(feature = "iterator")]
impl<'a, K, T> Map<'a, K, T>
where
    T: Serialize + DeserializeOwned,
    K: PrimaryKey<'a> + KeyDeserialize,
{
    /// while range_de assumes you set the prefix to one element and call range over the last one,
    /// prefix_range_de accepts bounds for the lowest and highest elements of the Prefix we wish to
    /// accept, and iterates over those. There are some issues that distinguish these to and blindly
    /// casting to Vec<u8> doesn't solve them.
    pub fn prefix_range_de<'c>(
        &self,
        store: &'c dyn Storage,
        min: Option<PrefixBound<'a, K::Prefix>>,
        max: Option<PrefixBound<'a, K::Prefix>>,
        order: cosmwasm_std::Order,
    ) -> Box<dyn Iterator<Item = StdResult<(K::Output, T)>> + 'c>
    where
        T: 'c,
        'a: 'c,
        K: 'c,
        K::Output: 'static,
    {
        let mapped = namespaced_prefix_range(store, self.namespace, min, max, order)
            .map(deserialize_kv::<K, T>);
        Box::new(mapped)
    }

    pub fn range_de<'c>(
        &self,
        store: &'c dyn Storage,
        min: Option<Bound>,
        max: Option<Bound>,
        order: cosmwasm_std::Order,
    ) -> Box<dyn Iterator<Item = StdResult<(K::Output, T)>> + 'c>
    where
        T: 'c,
        K::Output: 'static,
    {
        self.no_prefix_de().range_de(store, min, max, order)
    }

    pub fn keys_de<'c>(
        &self,
        store: &'c dyn Storage,
        min: Option<Bound>,
        max: Option<Bound>,
        order: cosmwasm_std::Order,
    ) -> Box<dyn Iterator<Item = StdResult<K::Output>> + 'c>
    where
        T: 'c,
        K::Output: 'static,
    {
        self.no_prefix_de().keys_de(store, min, max, order)
    }

    fn no_prefix_de(&self) -> Prefix<K, T> {
        Prefix::new(self.namespace, &[])
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use serde::{Deserialize, Serialize};
    use std::ops::Deref;

    #[cfg(feature = "iterator")]
    use crate::U32Key;
    use crate::U8Key;
    use cosmwasm_std::testing::MockStorage;
    #[cfg(feature = "iterator")]
    use cosmwasm_std::{Order, StdResult};

    #[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
    struct Data {
        pub name: String,
        pub age: i32,
    }

    const PEOPLE: Map<&[u8], Data> = Map::new("people");
    #[cfg(feature = "iterator")]
    const PEOPLE_ID: Map<U32Key, Data> = Map::new("people_id");

    const ALLOWANCE: Map<(&[u8], &[u8]), u64> = Map::new("allow");

    const TRIPLE: Map<(&[u8], U8Key, &str), u64> = Map::new("triple");

    #[test]
    fn create_path() {
        let path = PEOPLE.key(b"john");
        let key = path.deref();
        // this should be prefixed(people) || john
        assert_eq!("people".len() + "john".len() + 2, key.len());
        assert_eq!(b"people".to_vec().as_slice(), &key[2..8]);
        assert_eq!(b"john".to_vec().as_slice(), &key[8..]);

        let path = ALLOWANCE.key((b"john", b"maria"));
        let key = path.deref();
        // this should be prefixed(allow) || prefixed(john) || maria
        assert_eq!(
            "allow".len() + "john".len() + "maria".len() + 2 * 2,
            key.len()
        );
        assert_eq!(b"allow".to_vec().as_slice(), &key[2..7]);
        assert_eq!(b"john".to_vec().as_slice(), &key[9..13]);
        assert_eq!(b"maria".to_vec().as_slice(), &key[13..]);

        let path = TRIPLE.key((b"john", 8u8.into(), "pedro"));
        let key = path.deref();
        // this should be prefixed(allow) || prefixed(john) || maria
        assert_eq!(
            "triple".len() + "john".len() + 1 + "pedro".len() + 2 * 3,
            key.len()
        );
        assert_eq!(b"triple".to_vec().as_slice(), &key[2..8]);
        assert_eq!(b"john".to_vec().as_slice(), &key[10..14]);
        assert_eq!(8u8.to_be_bytes(), &key[16..17]);
        assert_eq!(b"pedro".to_vec().as_slice(), &key[17..]);
    }

    #[test]
    fn save_and_load() {
        let mut store = MockStorage::new();

        // save and load on one key
        let john = PEOPLE.key(b"john");
        let data = Data {
            name: "John".to_string(),
            age: 32,
        };
        assert_eq!(None, john.may_load(&store).unwrap());
        john.save(&mut store, &data).unwrap();
        assert_eq!(data, john.load(&store).unwrap());

        // nothing on another key
        assert_eq!(None, PEOPLE.may_load(&store, b"jack").unwrap());

        // same named path gets the data
        assert_eq!(data, PEOPLE.load(&store, b"john").unwrap());

        // removing leaves us empty
        john.remove(&mut store);
        assert_eq!(None, john.may_load(&store).unwrap());
    }

    #[test]
    fn existence() {
        let mut store = MockStorage::new();

        // set data in proper format
        let data = Data {
            name: "John".to_string(),
            age: 32,
        };
        PEOPLE.save(&mut store, b"john", &data).unwrap();

        // set and remove it
        PEOPLE.save(&mut store, b"removed", &data).unwrap();
        PEOPLE.remove(&mut store, b"removed");

        // invalid, but non-empty data
        store.set(&PEOPLE.key(b"random"), b"random-data");

        // any data, including invalid or empty is returned as "has"
        assert!(PEOPLE.has(&store, b"john"));
        assert!(PEOPLE.has(&store, b"random"));

        // if nothing was written, it is false
        assert!(!PEOPLE.has(&store, b"never-writen"));
        assert!(!PEOPLE.has(&store, b"removed"));
    }

    #[test]
    fn composite_keys() {
        let mut store = MockStorage::new();

        // save and load on a composite key
        let allow = ALLOWANCE.key((b"owner", b"spender"));
        assert_eq!(None, allow.may_load(&store).unwrap());
        allow.save(&mut store, &1234).unwrap();
        assert_eq!(1234, allow.load(&store).unwrap());

        // not under other key
        let different = ALLOWANCE.may_load(&store, (b"owners", b"pender")).unwrap();
        assert_eq!(None, different);

        // matches under a proper copy
        let same = ALLOWANCE.load(&store, (b"owner", b"spender")).unwrap();
        assert_eq!(1234, same);
    }

    #[test]
    fn triple_keys() {
        let mut store = MockStorage::new();

        // save and load on a triple composite key
        let triple = TRIPLE.key((b"owner", 10u8.into(), "recipient"));
        assert_eq!(None, triple.may_load(&store).unwrap());
        triple.save(&mut store, &1234).unwrap();
        assert_eq!(1234, triple.load(&store).unwrap());

        // not under other key
        let different = TRIPLE
            .may_load(&store, (b"owners", 10u8.into(), "ecipient"))
            .unwrap();
        assert_eq!(None, different);

        // matches under a proper copy
        let same = TRIPLE
            .load(&store, (b"owner", 10u8.into(), "recipient"))
            .unwrap();
        assert_eq!(1234, same);
    }

    #[test]
    #[cfg(feature = "iterator")]
    fn range_simple_key() {
        let mut store = MockStorage::new();

        // save and load on two keys
        let data = Data {
            name: "John".to_string(),
            age: 32,
        };
        PEOPLE.save(&mut store, b"john", &data).unwrap();

        let data2 = Data {
            name: "Jim".to_string(),
            age: 44,
        };
        PEOPLE.save(&mut store, b"jim", &data2).unwrap();

        // let's try to iterate!
        let all: StdResult<Vec<_>> = PEOPLE.range(&store, None, None, Order::Ascending).collect();
        let all = all.unwrap();
        assert_eq!(2, all.len());
        assert_eq!(
            all,
            vec![
                (b"jim".to_vec(), data2.clone()),
                (b"john".to_vec(), data.clone())
            ]
        );

        // let's try to iterate over a range
        let all: StdResult<Vec<_>> = PEOPLE
            .range(
                &store,
                Some(Bound::Inclusive(b"j".to_vec())),
                None,
                Order::Ascending,
            )
            .collect();
        let all = all.unwrap();
        assert_eq!(2, all.len());
        assert_eq!(
            all,
            vec![(b"jim".to_vec(), data2), (b"john".to_vec(), data.clone())]
        );

        // let's try to iterate over a more restrictive range
        let all: StdResult<Vec<_>> = PEOPLE
            .range(
                &store,
                Some(Bound::Inclusive(b"jo".to_vec())),
                None,
                Order::Ascending,
            )
            .collect();
        let all = all.unwrap();
        assert_eq!(1, all.len());
        assert_eq!(all, vec![(b"john".to_vec(), data)]);
    }

    #[test]
    #[cfg(feature = "iterator")]
    fn range_de_simple_string_key() {
        let mut store = MockStorage::new();

        // save and load on two keys
        let data = Data {
            name: "John".to_string(),
            age: 32,
        };
        PEOPLE.save(&mut store, b"john", &data).unwrap();

        let data2 = Data {
            name: "Jim".to_string(),
            age: 44,
        };
        PEOPLE.save(&mut store, b"jim", &data2).unwrap();

        // let's try to iterate!
        let all: StdResult<Vec<_>> = PEOPLE
            .range_de(&store, None, None, Order::Ascending)
            .collect();
        let all = all.unwrap();
        assert_eq!(2, all.len());
        assert_eq!(
            all,
            vec![
                (b"jim".to_vec(), data2.clone()),
                (b"john".to_vec(), data.clone())
            ]
        );

        // let's try to iterate over a range
        let all: StdResult<Vec<_>> = PEOPLE
            .range_de(
                &store,
                Some(Bound::Inclusive(b"j".to_vec())),
                None,
                Order::Ascending,
            )
            .collect();
        let all = all.unwrap();
        assert_eq!(2, all.len());
        assert_eq!(
            all,
            vec![(b"jim".to_vec(), data2), (b"john".to_vec(), data.clone())]
        );

        // let's try to iterate over a more restrictive range
        let all: StdResult<Vec<_>> = PEOPLE
            .range_de(
                &store,
                Some(Bound::Inclusive(b"jo".to_vec())),
                None,
                Order::Ascending,
            )
            .collect();
        let all = all.unwrap();
        assert_eq!(1, all.len());
        assert_eq!(all, vec![(b"john".to_vec(), data)]);
    }

    #[test]
    #[cfg(feature = "iterator")]
    fn range_de_simple_integer_key() {
        let mut store = MockStorage::new();

        // save and load on two keys
        let data = Data {
            name: "John".to_string(),
            age: 32,
        };
        PEOPLE_ID
            .save(&mut store, U32Key::new(1234), &data)
            .unwrap();

        let data2 = Data {
            name: "Jim".to_string(),
            age: 44,
        };
        PEOPLE_ID.save(&mut store, U32Key::new(56), &data2).unwrap();

        // let's try to iterate!
        let all: StdResult<Vec<_>> = PEOPLE_ID
            .range_de(&store, None, None, Order::Ascending)
            .collect();
        let all = all.unwrap();
        assert_eq!(2, all.len());
        assert_eq!(all, vec![(56, data2.clone()), (1234, data.clone())]);

        // let's try to iterate over a range
        let all: StdResult<Vec<_>> = PEOPLE_ID
            .range_de(
                &store,
                Some(Bound::Inclusive(U32Key::new(56).into())),
                None,
                Order::Ascending,
            )
            .collect();
        let all = all.unwrap();
        assert_eq!(2, all.len());
        assert_eq!(all, vec![(56, data2), (1234, data.clone())]);

        // let's try to iterate over a more restrictive range
        let all: StdResult<Vec<_>> = PEOPLE_ID
            .range_de(
                &store,
                Some(Bound::Inclusive(U32Key::new(57).into())),
                None,
                Order::Ascending,
            )
            .collect();
        let all = all.unwrap();
        assert_eq!(1, all.len());
        assert_eq!(all, vec![(1234, data)]);
    }

    #[test]
    #[cfg(feature = "iterator")]
    fn range_composite_key() {
        let mut store = MockStorage::new();

        // save and load on three keys, one under different owner
        ALLOWANCE
            .save(&mut store, (b"owner", b"spender"), &1000)
            .unwrap();
        ALLOWANCE
            .save(&mut store, (b"owner", b"spender2"), &3000)
            .unwrap();
        ALLOWANCE
            .save(&mut store, (b"owner2", b"spender"), &5000)
            .unwrap();

        // let's try to iterate!
        let all: StdResult<Vec<_>> = ALLOWANCE
            .range(&store, None, None, Order::Ascending)
            .collect();
        let all = all.unwrap();
        assert_eq!(3, all.len());
        assert_eq!(
            all,
            vec![
                ((b"owner".to_vec(), b"spender".to_vec()).joined_key(), 1000),
                ((b"owner".to_vec(), b"spender2".to_vec()).joined_key(), 3000),
                ((b"owner2".to_vec(), b"spender".to_vec()).joined_key(), 5000),
            ]
        );

        // let's try to iterate over a prefix
        let all: StdResult<Vec<_>> = ALLOWANCE
            .prefix(b"owner")
            .range(&store, None, None, Order::Ascending)
            .collect();
        let all = all.unwrap();
        assert_eq!(2, all.len());
        assert_eq!(
            all,
            vec![(b"spender".to_vec(), 1000), (b"spender2".to_vec(), 3000)]
        );
    }

    #[test]
    #[cfg(feature = "iterator")]
    fn range_de_composite_key() {
        let mut store = MockStorage::new();

        // save and load on three keys, one under different owner
        ALLOWANCE
            .save(&mut store, (b"owner", b"spender"), &1000)
            .unwrap();
        ALLOWANCE
            .save(&mut store, (b"owner", b"spender2"), &3000)
            .unwrap();
        ALLOWANCE
            .save(&mut store, (b"owner2", b"spender"), &5000)
            .unwrap();

        // let's try to iterate!
        let all: StdResult<Vec<_>> = ALLOWANCE
            .range_de(&store, None, None, Order::Ascending)
            .collect();
        let all = all.unwrap();
        assert_eq!(3, all.len());
        assert_eq!(
            all,
            vec![
                ((b"owner".to_vec(), b"spender".to_vec()), 1000),
                ((b"owner".to_vec(), b"spender2".to_vec()), 3000),
                ((b"owner2".to_vec(), b"spender".to_vec()), 5000)
            ]
        );

        // let's try to iterate over a prefix_de
        let all: StdResult<Vec<_>> = ALLOWANCE
            .prefix_de(b"owner")
            .range_de(&store, None, None, Order::Ascending)
            .collect();
        let all = all.unwrap();
        assert_eq!(2, all.len());
        assert_eq!(
            all,
            vec![(b"spender".to_vec(), 1000), (b"spender2".to_vec(), 3000),]
        );
    }

    #[test]
    #[cfg(feature = "iterator")]
    fn range_triple_key() {
        let mut store = MockStorage::new();

        // save and load on three keys, one under different owner
        TRIPLE
            .save(&mut store, (b"owner", 9u8.into(), "recipient"), &1000)
            .unwrap();
        TRIPLE
            .save(&mut store, (b"owner", 9u8.into(), "recipient2"), &3000)
            .unwrap();
        TRIPLE
            .save(&mut store, (b"owner", 10u8.into(), "recipient3"), &3000)
            .unwrap();
        TRIPLE
            .save(&mut store, (b"owner2", 9u8.into(), "recipient"), &5000)
            .unwrap();

        // let's try to iterate!
        let all: StdResult<Vec<_>> = TRIPLE.range(&store, None, None, Order::Ascending).collect();
        let all = all.unwrap();
        assert_eq!(4, all.len());
        assert_eq!(
            all,
            vec![
                (
                    (b"owner".to_vec(), U8Key::new(9), b"recipient".to_vec()).joined_key(),
                    1000
                ),
                (
                    (b"owner".to_vec(), U8Key::new(9), b"recipient2".to_vec()).joined_key(),
                    3000
                ),
                (
                    (b"owner".to_vec(), U8Key::new(10), b"recipient3".to_vec()).joined_key(),
                    3000
                ),
                (
                    (b"owner2".to_vec(), U8Key::new(9), b"recipient".to_vec()).joined_key(),
                    5000
                )
            ]
        );

        // let's iterate over a prefix
        let all: StdResult<Vec<_>> = TRIPLE
            .prefix((b"owner", 9u8.into()))
            .range(&store, None, None, Order::Ascending)
            .collect();
        let all = all.unwrap();
        assert_eq!(2, all.len());
        assert_eq!(
            all,
            vec![
                (b"recipient".to_vec(), 1000),
                (b"recipient2".to_vec(), 3000)
            ]
        );

        // let's iterate over a sub prefix
        let all: StdResult<Vec<_>> = TRIPLE
            .sub_prefix(b"owner")
            .range(&store, None, None, Order::Ascending)
            .collect();
        let all = all.unwrap();
        assert_eq!(3, all.len());
        // Use range_de() if you want key deserialization
        assert_eq!(
            all,
            vec![
                ((U8Key::new(9), b"recipient".to_vec()).joined_key(), 1000),
                ((U8Key::new(9), b"recipient2".to_vec()).joined_key(), 3000),
                ((U8Key::new(10), b"recipient3".to_vec()).joined_key(), 3000)
            ]
        );
    }

    #[test]
    #[cfg(feature = "iterator")]
    fn range_de_triple_key() {
        let mut store = MockStorage::new();

        // save and load on three keys, one under different owner
        TRIPLE
            .save(&mut store, (b"owner", 9u8.into(), "recipient"), &1000)
            .unwrap();
        TRIPLE
            .save(&mut store, (b"owner", 9u8.into(), "recipient2"), &3000)
            .unwrap();
        TRIPLE
            .save(&mut store, (b"owner", 10u8.into(), "recipient3"), &3000)
            .unwrap();
        TRIPLE
            .save(&mut store, (b"owner2", 9u8.into(), "recipient"), &5000)
            .unwrap();

        // let's try to iterate!
        let all: StdResult<Vec<_>> = TRIPLE
            .range_de(&store, None, None, Order::Ascending)
            .collect();
        let all = all.unwrap();
        assert_eq!(4, all.len());
        assert_eq!(
            all,
            vec![
                ((b"owner".to_vec(), 9, "recipient".to_string()), 1000),
                ((b"owner".to_vec(), 9, "recipient2".to_string()), 3000),
                ((b"owner".to_vec(), 10, "recipient3".to_string()), 3000),
                ((b"owner2".to_vec(), 9, "recipient".to_string()), 5000)
            ]
        );

        // let's iterate over a sub_prefix_de
        let all: StdResult<Vec<_>> = TRIPLE
            .sub_prefix_de(b"owner")
            .range_de(&store, None, None, Order::Ascending)
            .collect();
        let all = all.unwrap();
        assert_eq!(3, all.len());
        assert_eq!(
            all,
            vec![
                ((9, "recipient".to_string()), 1000),
                ((9, "recipient2".to_string()), 3000),
                ((10, "recipient3".to_string()), 3000),
            ]
        );

        // let's iterate over a prefix_de
        let all: StdResult<Vec<_>> = TRIPLE
            .prefix_de((b"owner", U8Key::new(9)))
            .range_de(&store, None, None, Order::Ascending)
            .collect();
        let all = all.unwrap();
        assert_eq!(2, all.len());
        assert_eq!(
            all,
            vec![
                ("recipient".to_string(), 1000),
                ("recipient2".to_string(), 3000),
            ]
        );
    }

    #[test]
    fn basic_update() {
        let mut store = MockStorage::new();

        let add_ten = |a: Option<u64>| -> StdResult<_> { Ok(a.unwrap_or_default() + 10) };

        // save and load on three keys, one under different owner
        let key: (&[u8], &[u8]) = (b"owner", b"spender");
        ALLOWANCE.update(&mut store, key, add_ten).unwrap();
        let twenty = ALLOWANCE.update(&mut store, key, add_ten).unwrap();
        assert_eq!(20, twenty);
        let loaded = ALLOWANCE.load(&store, key).unwrap();
        assert_eq!(20, loaded);
    }

    #[test]
    fn readme_works() -> StdResult<()> {
        let mut store = MockStorage::new();
        let data = Data {
            name: "John".to_string(),
            age: 32,
        };

        // load and save with extra key argument
        let empty = PEOPLE.may_load(&store, b"john")?;
        assert_eq!(None, empty);
        PEOPLE.save(&mut store, b"john", &data)?;
        let loaded = PEOPLE.load(&store, b"john")?;
        assert_eq!(data, loaded);

        // nothing on another key
        let missing = PEOPLE.may_load(&store, b"jack")?;
        assert_eq!(None, missing);

        // update function for new or existing keys
        let birthday = |d: Option<Data>| -> StdResult<Data> {
            match d {
                Some(one) => Ok(Data {
                    name: one.name,
                    age: one.age + 1,
                }),
                None => Ok(Data {
                    name: "Newborn".to_string(),
                    age: 0,
                }),
            }
        };

        let old_john = PEOPLE.update(&mut store, b"john", birthday)?;
        assert_eq!(33, old_john.age);
        assert_eq!("John", old_john.name.as_str());

        let new_jack = PEOPLE.update(&mut store, b"jack", birthday)?;
        assert_eq!(0, new_jack.age);
        assert_eq!("Newborn", new_jack.name.as_str());

        // update also changes the store
        assert_eq!(old_john, PEOPLE.load(&store, b"john")?);
        assert_eq!(new_jack, PEOPLE.load(&store, b"jack")?);

        // removing leaves us empty
        PEOPLE.remove(&mut store, b"john");
        let empty = PEOPLE.may_load(&store, b"john")?;
        assert_eq!(None, empty);

        Ok(())
    }

    #[test]
    fn readme_works_composite_keys() -> StdResult<()> {
        let mut store = MockStorage::new();

        // save and load on a composite key
        let empty = ALLOWANCE.may_load(&store, (b"owner", b"spender"))?;
        assert_eq!(None, empty);
        ALLOWANCE.save(&mut store, (b"owner", b"spender"), &777)?;
        let loaded = ALLOWANCE.load(&store, (b"owner", b"spender"))?;
        assert_eq!(777, loaded);

        // doesn't appear under other key (even if a concat would be the same)
        let different = ALLOWANCE.may_load(&store, (b"owners", b"pender")).unwrap();
        assert_eq!(None, different);

        // simple update
        ALLOWANCE.update(&mut store, (b"owner", b"spender"), |v| -> StdResult<u64> {
            Ok(v.unwrap_or_default() + 222)
        })?;
        let loaded = ALLOWANCE.load(&store, (b"owner", b"spender"))?;
        assert_eq!(999, loaded);

        Ok(())
    }

    #[test]
    fn readme_works_with_path() -> StdResult<()> {
        let mut store = MockStorage::new();
        let data = Data {
            name: "John".to_string(),
            age: 32,
        };

        // create a Path one time to use below
        let john = PEOPLE.key(b"john");

        // Use this just like an Item above
        let empty = john.may_load(&store)?;
        assert_eq!(None, empty);
        john.save(&mut store, &data)?;
        let loaded = john.load(&store)?;
        assert_eq!(data, loaded);
        john.remove(&mut store);
        let empty = john.may_load(&store)?;
        assert_eq!(None, empty);

        // same for composite keys, just use both parts in key()
        let allow = ALLOWANCE.key((b"owner", b"spender"));
        allow.save(&mut store, &1234)?;
        let loaded = allow.load(&store)?;
        assert_eq!(1234, loaded);
        allow.update(&mut store, |x| -> StdResult<u64> {
            Ok(x.unwrap_or_default() * 2)
        })?;
        let loaded = allow.load(&store)?;
        assert_eq!(2468, loaded);

        Ok(())
    }

    #[test]
    #[cfg(feature = "iterator")]
    fn readme_with_range() -> StdResult<()> {
        let mut store = MockStorage::new();

        // save and load on two keys
        let data = Data {
            name: "John".to_string(),
            age: 32,
        };
        PEOPLE.save(&mut store, b"john", &data)?;
        let data2 = Data {
            name: "Jim".to_string(),
            age: 44,
        };
        PEOPLE.save(&mut store, b"jim", &data2)?;

        // iterate over them all
        let all: StdResult<Vec<_>> = PEOPLE.range(&store, None, None, Order::Ascending).collect();
        assert_eq!(
            all?,
            vec![(b"jim".to_vec(), data2), (b"john".to_vec(), data.clone())]
        );

        // or just show what is after jim
        let all: StdResult<Vec<_>> = PEOPLE
            .range(
                &store,
                Some(Bound::Exclusive(b"jim".to_vec())),
                None,
                Order::Ascending,
            )
            .collect();
        assert_eq!(all?, vec![(b"john".to_vec(), data)]);

        // save and load on three keys, one under different owner
        ALLOWANCE.save(&mut store, (b"owner", b"spender"), &1000)?;
        ALLOWANCE.save(&mut store, (b"owner", b"spender2"), &3000)?;
        ALLOWANCE.save(&mut store, (b"owner2", b"spender"), &5000)?;

        // get all under one key
        let all: StdResult<Vec<_>> = ALLOWANCE
            .prefix(b"owner")
            .range(&store, None, None, Order::Ascending)
            .collect();
        assert_eq!(
            all?,
            vec![(b"spender".to_vec(), 1000), (b"spender2".to_vec(), 3000)]
        );

        // Or ranges between two items (even reverse)
        let all: StdResult<Vec<_>> = ALLOWANCE
            .prefix(b"owner")
            .range(
                &store,
                Some(Bound::Exclusive(b"spender1".to_vec())),
                Some(Bound::Inclusive(b"spender2".to_vec())),
                Order::Descending,
            )
            .collect();
        assert_eq!(all?, vec![(b"spender2".to_vec(), 3000)]);

        Ok(())
    }

    #[test]
    #[cfg(feature = "iterator")]
    fn prefixed_range_works() {
        // this is designed to look as much like a secondary index as possible
        // we want to query over a range of u32 for the first key and all subkeys
        const AGES: Map<(U32Key, Vec<u8>), u64> = Map::new("ages");

        let mut store = MockStorage::new();
        AGES.save(&mut store, (2.into(), vec![1, 2, 3]), &123)
            .unwrap();
        AGES.save(&mut store, (3.into(), vec![4, 5, 6]), &456)
            .unwrap();
        AGES.save(&mut store, (5.into(), vec![7, 8, 9]), &789)
            .unwrap();
        AGES.save(&mut store, (5.into(), vec![9, 8, 7]), &987)
            .unwrap();
        AGES.save(&mut store, (7.into(), vec![20, 21, 22]), &2002)
            .unwrap();
        AGES.save(&mut store, (8.into(), vec![23, 24, 25]), &2332)
            .unwrap();

        // typical range under one prefix as a control
        let fives = AGES
            .prefix(5.into())
            .range(&store, None, None, Order::Ascending)
            .collect::<StdResult<Vec<_>>>()
            .unwrap();
        assert_eq!(fives.len(), 2);
        assert_eq!(fives, vec![(vec![7, 8, 9], 789), (vec![9, 8, 7], 987)]);

        let keys: Vec<_> = AGES
            .no_prefix()
            .keys(&store, None, None, Order::Ascending)
            .collect();
        println!("keys: {:?}", keys);

        // using inclusive bounds both sides
        let include = AGES
            .prefix_range(
                &store,
                Some(PrefixBound::inclusive(3)),
                Some(PrefixBound::inclusive(7)),
                Order::Ascending,
            )
            .map(|r| r.map(|(_, v)| v))
            .collect::<StdResult<Vec<_>>>()
            .unwrap();
        assert_eq!(include.len(), 4);
        assert_eq!(include, vec![456, 789, 987, 2002]);

        // using exclusive bounds both sides
        let exclude = AGES
            .prefix_range(
                &store,
                Some(PrefixBound::exclusive(3)),
                Some(PrefixBound::exclusive(7)),
                Order::Ascending,
            )
            .map(|r| r.map(|(_, v)| v))
            .collect::<StdResult<Vec<_>>>()
            .unwrap();
        assert_eq!(exclude.len(), 2);
        assert_eq!(exclude, vec![789, 987]);

        // using inclusive in descending
        let include = AGES
            .prefix_range(
                &store,
                Some(PrefixBound::inclusive(3)),
                Some(PrefixBound::inclusive(5)),
                Order::Descending,
            )
            .map(|r| r.map(|(_, v)| v))
            .collect::<StdResult<Vec<_>>>()
            .unwrap();
        assert_eq!(include.len(), 3);
        assert_eq!(include, vec![987, 789, 456]);

        // using exclusive in descending
        let include = AGES
            .prefix_range(
                &store,
                Some(PrefixBound::exclusive(2)),
                Some(PrefixBound::exclusive(5)),
                Order::Descending,
            )
            .map(|r| r.map(|(_, v)| v))
            .collect::<StdResult<Vec<_>>>()
            .unwrap();
        assert_eq!(include.len(), 1);
        assert_eq!(include, vec![456]);
    }

    #[test]
    #[cfg(feature = "iterator")]
    fn prefixed_range_de_works() {
        // this is designed to look as much like a secondary index as possible
        // we want to query over a range of u32 for the first key and all subkeys
        const AGES: Map<(U32Key, &str), u64> = Map::new("ages");

        let mut store = MockStorage::new();
        AGES.save(&mut store, (2.into(), "123"), &123).unwrap();
        AGES.save(&mut store, (3.into(), "456"), &456).unwrap();
        AGES.save(&mut store, (5.into(), "789"), &789).unwrap();
        AGES.save(&mut store, (5.into(), "987"), &987).unwrap();
        AGES.save(&mut store, (7.into(), "202122"), &2002).unwrap();
        AGES.save(&mut store, (8.into(), "232425"), &2332).unwrap();

        // typical range under one prefix as a control
        let fives = AGES
            .prefix_de(5.into())
            .range(&store, None, None, Order::Ascending)
            .collect::<StdResult<Vec<_>>>()
            .unwrap();
        assert_eq!(fives.len(), 2);
        assert_eq!(
            fives,
            vec![("789".to_string(), 789), ("987".to_string(), 987)]
        );

        let keys: Vec<_> = AGES
            .no_prefix()
            .keys_de(&store, None, None, Order::Ascending)
            .collect();
        println!("keys: {:?}", keys);

        // using inclusive bounds both sides
        let include = AGES
            .prefix_range_de(
                &store,
                Some(PrefixBound::inclusive(3)),
                Some(PrefixBound::inclusive(7)),
                Order::Ascending,
            )
            .map(|r| r.map(|(_, v)| v))
            .collect::<StdResult<Vec<_>>>()
            .unwrap();
        assert_eq!(include.len(), 4);
        assert_eq!(include, vec![456, 789, 987, 2002]);

        // using exclusive bounds both sides
        let exclude = AGES
            .prefix_range_de(
                &store,
                Some(PrefixBound::exclusive(3)),
                Some(PrefixBound::exclusive(7)),
                Order::Ascending,
            )
            .map(|r| r.map(|(_, v)| v))
            .collect::<StdResult<Vec<_>>>()
            .unwrap();
        assert_eq!(exclude.len(), 2);
        assert_eq!(exclude, vec![789, 987]);

        // using inclusive in descending
        let include = AGES
            .prefix_range_de(
                &store,
                Some(PrefixBound::inclusive(3)),
                Some(PrefixBound::inclusive(5)),
                Order::Descending,
            )
            .map(|r| r.map(|(_, v)| v))
            .collect::<StdResult<Vec<_>>>()
            .unwrap();
        assert_eq!(include.len(), 3);
        assert_eq!(include, vec![987, 789, 456]);

        // using exclusive in descending
        let include = AGES
            .prefix_range_de(
                &store,
                Some(PrefixBound::exclusive(2)),
                Some(PrefixBound::exclusive(5)),
                Order::Descending,
            )
            .map(|r| r.map(|(_, v)| v))
            .collect::<StdResult<Vec<_>>>()
            .unwrap();
        assert_eq!(include.len(), 1);
        assert_eq!(include, vec![456]);
    }
}
