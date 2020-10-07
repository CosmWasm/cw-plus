// this module requires iterator to be useful at all
#![cfg(feature = "iterator")]

use cosmwasm_std::{Order, StdResult, Storage, KV};
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::iter_helpers::range_with_prefix;
use crate::map::OwnedMap;

/// MARKER is stored in the multi-index as value, but we only look at the key (which is pk)
const MARKER: &[u8] = b"1";

pub fn index_string(data: &str) -> Vec<u8> {
    data.as_bytes().to_vec()
}

// Look at https://docs.rs/endiannezz/0.4.1/endiannezz/trait.Primitive.html
// if you want to make this generic over all ints
pub fn index_u64(data: u64) -> Vec<u8> {
    data.to_be_bytes().into()
}

pub fn index_i32(data: i32) -> Vec<u8> {
    data.to_be_bytes().into()
}

// 2 main variants:
//  * store (namespace, index_name, idx_value, key) -> b"1" - allows many and references pk
//  * store (namespace, index_name, idx_value) -> {key, value} - allows one and copies pk and data
//  // this would be the primary key - we abstract that too???
//  * store (namespace, index_name, pk) -> value - allows one with data
pub trait Index<T>
where
    T: Serialize + DeserializeOwned + Clone,
{
    // TODO: do we make this any Vec<u8> ?
    fn name(&self) -> String;
    fn index(&self, data: &T) -> Vec<u8>;

    // TODO: pk: PrimaryKey not just &[u8] ???

    fn save<S: Storage>(&self, store: &mut S, pk: &[u8], data: &T) -> StdResult<()>;
    fn remove<S: Storage>(&self, store: &mut S, pk: &[u8], old_data: &T) -> StdResult<()>;

    // these should be implemented by all
    fn pks_by_index<'c, S: Storage>(
        &self,
        store: &'c S,
        idx: &[u8],
    ) -> Box<dyn Iterator<Item = Vec<u8>> + 'c>;

    /// returns all items that match this secondary index, always by pk Ascending
    fn items_by_index<'c, S: Storage>(
        &'c self,
        store: &'c S,
        idx: &[u8],
    ) -> Box<dyn Iterator<Item = StdResult<KV<T>>> + 'c>;

    // TODO: range over secondary index values? (eg. all results with 30 < age < 40)
}

pub struct MultiIndex<'a, T>
where
    T: Serialize + DeserializeOwned + Clone,
{
    idx_fn: fn(&T) -> Vec<u8>,
    _name: &'a str,
    idx_map: OwnedMap<'a, (&'a [u8], &'a [u8]), Vec<u8>>,
    pk_map: OwnedMap<'a, &'a [u8], T>,
}

impl<'a, T> MultiIndex<'a, T>
where
    T: Serialize + DeserializeOwned + Clone,
{
    // TODO: review this constructor and how to build the pk_map
    pub fn new(idx_fn: fn(&T) -> Vec<u8>, namespace: &'a [u8], name: &'a str) -> Self {
        MultiIndex {
            idx_fn,
            idx_map: OwnedMap::new(vec![namespace, name.as_bytes()]),
            pk_map: OwnedMap::new(vec![namespace, b"_pk"]),
            _name: name,
        }
    }
}

impl<'a, T> Index<T> for MultiIndex<'a, T>
where
    T: Serialize + DeserializeOwned + Clone,
{
    fn name(&self) -> String {
        self._name.to_string()
    }

    fn index(&self, data: &T) -> Vec<u8> {
        (self.idx_fn)(data)
    }

    fn save<S: Storage>(&self, store: &mut S, pk: &[u8], data: &T) -> StdResult<()> {
        let idx = self.index(data);
        let key = self.idx_map.to_map().key((&idx, &pk));
        store.set(&key, MARKER);
        Ok(())
    }

    fn remove<S: Storage>(&self, store: &mut S, pk: &[u8], old_data: &T) -> StdResult<()> {
        let idx = self.index(old_data);
        let key = self.idx_map.to_map().key((&idx, &pk));
        store.remove(&key);
        Ok(())
    }

    fn pks_by_index<'c, S: Storage>(
        &self,
        store: &'c S,
        idx: &[u8],
    ) -> Box<dyn Iterator<Item = Vec<u8>> + 'c> {
        let prefix = self.idx_map.to_map().prefix(idx);
        let mapped =
            range_with_prefix(store, &prefix, None, None, Order::Ascending).map(|(k, _)| k);
        Box::new(mapped)
    }

    /// returns all items that match this secondary index, always by pk Ascending
    fn items_by_index<'c, S: Storage>(
        &'c self,
        store: &'c S,
        idx: &[u8],
    ) -> Box<dyn Iterator<Item = StdResult<KV<T>>> + 'c> {
        let mapped = self.pks_by_index(store, idx).map(move |pk| {
            let v = self.pk_map.to_map().load(store, &pk)?;
            Ok((pk, v))
        });
        Box::new(mapped)
    }
}

// #[derive(Deserialize, Serialize, Clone)]
// pub(crate) struct UniqueRef<T: Clone> {
//     pk: Binary,
//     value: T,
// }
//
// pub(crate) struct UniqueIndex<'a, T>
// where
//     T: Serialize + DeserializeOwned + Clone,
// {
//     idx_fn: fn(&T) -> Vec<u8>,
//     _name: &'a str,
// }
//
// impl<'a, T> UniqueIndex<'a, T>
// where
//     T: Serialize + DeserializeOwned + Clone,
// {
//     pub fn new(idx_fn: fn(&T) -> Vec<u8>, name: &'a str) -> Self {
//         UniqueIndex {
//             idx_fn,
//             _name: name,
//         }
//     }
// }

// impl<'a, S, T> Index<S, T> for UniqueIndex<'a, S, T>
// where
//     S: Storage,
//     T: Serialize + DeserializeOwned + Clone,
// {
//     fn name(&self) -> String {
//         self._name.to_string()
//     }
//
//     fn index(&self, data: &T) -> Vec<u8> {
//         (self.idx_fn)(data)
//     }
//
//     // we store (namespace, index_name, idx_value) -> { pk, value }
//     fn insert(&self, map: &mut Map<K, T>, pk: &[u8], data: &T) -> StdResult<()> {
//         let idx = self.index(data);
//         let key = map.build_secondary_key(&[self._name.as_bytes()], &idx);
//         // error if this is already set
//         if map.storage.get(&key).is_some() {
//             return Err(StdError::generic_err(format!(
//                 "Violates unique constraint on index `{}`",
//                 self._name
//             )));
//         }
//
//         let reference = UniqueRef::<T> {
//             pk: pk.into(),
//             value: data.clone(),
//         };
//         map.storage.set(&key, &to_vec(&reference)?);
//         Ok(())
//     }
//
//     // we store (namespace, index_name, idx_value) -> { pk, value }
//     fn remove(&self, map: &mut Map<K, T>, _pk: &[u8], old_data: &T) -> StdResult<()> {
//         let idx = self.index(old_data);
//         let key = map.build_secondary_key(&[self._name.as_bytes()], &idx);
//         map.storage.remove(&key);
//         Ok(())
//     }
//
//     // there is exactly 0 or 1 here...
//     fn pks_by_index<'c>(
//         &self,
//         map: &'c Map<K, T>,
//         idx: &[u8],
//     ) -> Box<dyn Iterator<Item = Vec<u8>> + 'c> {
//         // TODO: update types to return StdResult<Vec<u8>> ?
//         // should never really happen, but I dislike unwrap
//         let mapped = self.items_by_index(map, idx).map(|res| res.unwrap().0);
//         Box::new(mapped)
//     }
//
//     /// returns all items that match this secondary index, always by pk Ascending
//     fn items_by_index<'c>(
//         &self,
//         map: &'c Map<K, T>,
//         idx: &[u8],
//     ) -> Box<dyn Iterator<Item = StdResult<KV<T>>> + 'c> {
//         let key = map.build_secondary_key(&[self._name.as_bytes()], &idx);
//         let data = match map.storage.get(&key) {
//             Some(bin) => vec![bin],
//             None => vec![],
//         };
//         let mapped = data.into_iter().map(|bin| {
//             let parsed: UniqueRef<T> = from_slice(&bin)?;
//             Ok((parsed.pk.into_vec(), parsed.value))
//         });
//         Box::new(mapped)
//     }
// }
