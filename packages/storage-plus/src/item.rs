use serde::de::DeserializeOwned;
use serde::Serialize;
use std::marker::PhantomData;

use cosmwasm_std::{to_vec, StdError, StdResult, Storage};

use crate::helpers::{may_deserialize, must_deserialize};

/// Item stores one typed item at the given key.
/// This is an analog of Singleton.
/// It functions just as Path but doesn't ue a Vec and thus has a const fn constructor.
pub struct Item<'a, T> {
    // this is full key - no need to length-prefix it, we only store one item
    storage_key: &'a [u8],
    // see https://doc.rust-lang.org/std/marker/struct.PhantomData.html#unused-type-parameters for why this is needed
    data_type: PhantomData<T>,
}

impl<'a, T> Item<'a, T> {
    pub const fn new(storage_key: &'a [u8]) -> Self {
        Item {
            storage_key,
            data_type: PhantomData,
        }
    }
}

impl<'a, T> Item<'a, T>
where
    T: Serialize + DeserializeOwned,
{
    /// save will serialize the model and store, returns an error on serialization issues
    pub fn save<S: Storage>(&self, store: &mut S, data: &T) -> StdResult<()> {
        store.set(self.storage_key, &to_vec(data)?);
        Ok(())
    }

    pub fn remove<S: Storage>(&self, store: &mut S) {
        store.remove(self.storage_key);
    }

    /// load will return an error if no data is set at the given key, or on parse error
    pub fn load<S: Storage>(&self, store: &S) -> StdResult<T> {
        let value = store.get(self.storage_key);
        must_deserialize(&value)
    }

    /// may_load will parse the data stored at the key if present, returns Ok(None) if no data there.
    /// returns an error on issues parsing
    pub fn may_load<S: Storage>(&self, store: &S) -> StdResult<Option<T>> {
        let value = store.get(self.storage_key);
        may_deserialize(&value)
    }

    /// Loads the data, perform the specified action, and store the result
    /// in the database. This is shorthand for some common sequences, which may be useful.
    ///
    /// If the data exists, `action(Some(value))` is called. Otherwise `action(None)` is called.
    pub fn update<A, E, S>(&mut self, store: &mut S, action: A) -> Result<T, E>
    where
        A: FnOnce(Option<T>) -> Result<T, E>,
        E: From<StdError>,
        S: Storage,
    {
        let input = self.may_load(store)?;
        let output = action(input)?;
        self.save(store, &output)?;
        Ok(output)
    }
}
