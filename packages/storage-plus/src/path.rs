use serde::de::DeserializeOwned;
use serde::Serialize;
use std::marker::PhantomData;

use crate::helpers::{may_deserialize, must_deserialize, nested_namespaces_with_key};
use crate::keys::Key;
use cosmwasm_std::{to_vec, StdError, StdResult, Storage};
use std::ops::Deref;

#[derive(Debug, Clone)]
pub struct Path<T>
where
    T: Serialize + DeserializeOwned,
{
    /// all namespaces prefixes and concatenated with the key
    pub(crate) storage_key: Vec<u8>,
    // see https://doc.rust-lang.org/std/marker/struct.PhantomData.html#unused-type-parameters for why this is needed
    data: PhantomData<T>,
}

impl<T> Deref for Path<T>
where
    T: Serialize + DeserializeOwned,
{
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        &self.storage_key
    }
}

impl<T> Path<T>
where
    T: Serialize + DeserializeOwned,
{
    pub fn new(namespace: &[u8], keys: &[&[u8]]) -> Self {
        let l = keys.len();
        // FIXME: make this more efficient
        let storage_key = nested_namespaces_with_key(
            &[namespace],
            &keys[0..l - 1]
                .iter()
                .map(|k| Key::Ref(k))
                .collect::<Vec<Key>>(),
            keys[l - 1],
        );
        Path {
            storage_key,
            data: PhantomData,
        }
    }

    /// save will serialize the model and store, returns an error on serialization issues
    pub fn save(&self, store: &mut dyn Storage, data: &T) -> StdResult<()> {
        store.set(&self.storage_key, &to_vec(data)?);
        Ok(())
    }

    pub fn remove(&self, store: &mut dyn Storage) {
        store.remove(&self.storage_key);
    }

    /// load will return an error if no data is set at the given key, or on parse error
    pub fn load(&self, store: &dyn Storage) -> StdResult<T> {
        let value = store.get(&self.storage_key);
        must_deserialize(&value)
    }

    /// may_load will parse the data stored at the key if present, returns Ok(None) if no data there.
    /// returns an error on issues parsing
    pub fn may_load(&self, store: &dyn Storage) -> StdResult<Option<T>> {
        let value = store.get(&self.storage_key);
        may_deserialize(&value)
    }

    /// has returns true or false if any data is at this key, without parsing or interpreting the
    /// contents. It will returns true for an length-0 byte array (Some(b"")), if you somehow manage to set that.
    pub fn has(&self, store: &dyn Storage) -> bool {
        store.get(&self.storage_key).is_some()
    }

    /// Loads the data, perform the specified action, and store the result
    /// in the database. This is shorthand for some common sequences, which may be useful.
    ///
    /// If the data exists, `action(Some(value))` is called. Otherwise `action(None)` is called.
    pub fn update<A, E>(&self, store: &mut dyn Storage, action: A) -> Result<T, E>
    where
        A: FnOnce(Option<T>) -> Result<T, E>,
        E: From<StdError>,
    {
        let input = self.may_load(store)?;
        let output = action(input)?;
        self.save(store, &output)?;
        Ok(output)
    }
}
