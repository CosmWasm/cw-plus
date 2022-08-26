//! An "Stack" is a storage wrapper that guarantees constant-cost appending to and popping
//! from a list of items in storage.
//!
//! This is achieved by storing each item in a separate storage entry. A special key is reserved
//! for storing the length of the collection so far.
use std::any::type_name;
use std::convert::TryInto;
use std::marker::PhantomData;

use serde::{de::DeserializeOwned, Serialize};

use cosmwasm_std::{to_vec, StdError, StdResult, Storage};

use crate::helpers::must_deserialize;

const LEN_KEY: &[u8] = b"len";

pub struct Stack<'a, T>
where
    T: Serialize + DeserializeOwned,
{
    /// prefix of the newly constructed Storage
    namespace: &'a [u8],
    /// needed if any suffixes were added to the original namespace.
    /// therefore it is not necessarily same as the namespace.
    prefix: Option<Vec<u8>>,
    length: Option<u32>,
    item_type: PhantomData<T>,
}

impl<'a, T: Serialize + DeserializeOwned> Stack<'a, T> {
    /// constructor
    pub const fn new(prefix: &'a str) -> Self {
        Self {
            namespace: prefix.as_bytes(),
            prefix: None,
            length: None,
            item_type: PhantomData,
        }
    }
    /// This is used to produce a new Stack. This can be used when you want to associate an Stack to each user
    /// and you still get to define the Stack as a static constant
    pub fn add_suffix(&self, suffix: &str) -> Self {
        let prefix = if let Some(prefix) = &self.prefix {
            [prefix.clone(), suffix.as_bytes().to_vec()].concat()
        } else {
            [self.namespace.to_vec(), suffix.as_bytes().to_vec()].concat()
        };
        Self {
            namespace: self.namespace,
            prefix: Some(prefix),
            length: None,
            item_type: self.item_type,
        }
    }
}

impl<'a, T: Serialize + DeserializeOwned> Stack<'a, T> {
    /// gets the length from storage, and otherwise sets it to 0
    pub fn len(&self, storage: &dyn Storage) -> StdResult<u32> {
        let may_len = self.length;
        match may_len {
            Some(len) => Ok(len),
            None => {
                let len_key = [self.as_slice(), LEN_KEY].concat();
                if let Some(len_vec) = storage.get(&len_key) {
                    let len_bytes = len_vec
                        .as_slice()
                        .try_into()
                        .map_err(|err| StdError::parse_err("u32", err))?;
                    let len = u32::from_be_bytes(len_bytes);
                    Ok(len)
                } else {
                    Ok(0)
                }
            }
        }
    }
    /// checks if the collection has any elements
    pub fn is_empty(&self, storage: &dyn Storage) -> StdResult<bool> {
        Ok(self.get_len(storage)? == 0)
    }
    /// gets the element at pos if within bounds
    pub fn get_at(&self, storage: &dyn Storage, pos: u32) -> Option<T> {
        let len = self.get_len(storage).unwrap();
        if pos > len {
            return None;
        }
        Some(self.get_at_unchecked(storage, pos).unwrap())
    }
    /// tries to get the element at pos
    fn get_at_unchecked(&self, storage: &dyn Storage, pos: u32) -> StdResult<T> {
        let key = pos.to_be_bytes();
        self.load_impl(storage, &key)
    }

    /// Set the length of the collection
    fn set_len(&self, storage: &mut dyn Storage, len: u32) {
        let len_key = [self.as_slice(), LEN_KEY].concat();
        storage.set(&len_key, &len.to_be_bytes());

        let mut may_len = self.length;
        may_len = Some(len);
    }
    /// Clear the collection
    pub fn clear(&self, storage: &mut dyn Storage) {
        let len_obj = self.get_len(storage);
        let mut len: u32 = 0;
        if len_obj.is_ok() {
            len = len_obj.unwrap();
        }

        for i in (0..len - 1).rev() {
            self.remove_at(storage, i).unwrap();
        }

        self.set_len(storage, 0);
    }
    /// Replaces data at a position within bounds
    pub fn set_at(&self, storage: &mut dyn Storage, pos: u32, item: &T) -> StdResult<()> {
        let len = self.get_len(storage)?;
        if pos >= len {
            return Err(StdError::generic_err("Stack access out of bounds"));
        }
        self.set_at_unchecked(storage, pos, item)
    }
    /// Sets data at a given index
    fn set_at_unchecked(&self, storage: &mut dyn Storage, pos: u32, item: &T) -> StdResult<()> {
        self.save_impl(storage, &pos.to_be_bytes(), item)
    }

    fn remove_at(&self, storage: &mut dyn Storage, pos: u32) -> StdResult<()> {
        self.remove_impl(storage, &pos.to_be_bytes())
    }
    /// Pushes an item to Stack
    pub fn push(&self, storage: &mut dyn Storage, item: &T) -> StdResult<()> {
        let len = self.get_len(storage)?;
        self.set_at_unchecked(storage, len, item)?;
        self.set_len(storage, len + 1);
        Ok(())
    }
    /// Pops an item from Stack
    pub fn pop(&self, storage: &mut dyn Storage) -> Option<T> {
        if let Some(len) = self.get_len(storage).unwrap().checked_sub(1) {
            let item = self.get_at_unchecked(storage, len).unwrap();
            self.remove_at(storage, len).unwrap();
            self.set_len(storage, len);
            Some(item)
        } else {
            None
        }
    }

    /// Returns a readonly iterator
    pub fn iter(&self, storage: &'a dyn Storage) -> StdResult<StackIter<T>> {
        let len = self.get_len(storage)?;
        let iter = StackIter::new(self, storage, 0, len);
        Ok(iter)
    }
    /// does paging with the given parameters
    pub fn paging(&self, storage: &dyn Storage, start_page: u32, size: u32) -> StdResult<Vec<T>> {
        self.iter(storage)?
            .skip((start_page as usize) * (size as usize))
            .take(size as usize)
            .collect()
    }
}

impl<'a, T: Serialize + DeserializeOwned> Clone for Stack<'a, T> {
    fn clone(&self) -> Self {
        Self {
            namespace: self.namespace.clone(),
            prefix: self.prefix.clone(),
            length: None,
            item_type: self.item_type.clone(),
        }
    }
}

impl<'a, T: Serialize + DeserializeOwned> Stack<'a, T> {
    fn as_slice(&self) -> &[u8] {
        if let Some(prefix) = &self.prefix {
            prefix
        } else {
            self.namespace
        }
    }

    /// Returns StdResult<T> from retrieving the item with the specified key.  Returns a
    /// StdError::NotFound if there is no item with that key
    ///
    /// # Arguments
    ///
    /// * `storage` - a reference to the storage this item is in
    /// * `key` - a byte slice representing the key to access the stored item
    fn load_impl(&self, storage: &dyn Storage, key: &[u8]) -> StdResult<T> {
        let prefixed_key = [self.as_slice(), key].concat();
        must_deserialize(&Some(
            storage
                .get(&prefixed_key)
                .ok_or(StdError::not_found(type_name::<T>()))
                .unwrap(),
        ))
    }

    /// Returns StdResult<()> resulting from saving an item to storage
    ///
    /// # Arguments
    ///
    /// * `storage` - a mutable reference to the storage this item should go to
    /// * `key` - a byte slice representing the key to access the stored item
    /// * `value` - a reference to the item to store
    fn save_impl(&self, storage: &mut dyn Storage, key: &[u8], value: &T) -> StdResult<()> {
        let prefixed_key = [self.as_slice(), key].concat();
        storage.set(&prefixed_key, &to_vec(value)?);
        Ok(())
    }

    /// Returns StdResult<()> resulting from saving an item to storage
    ///
    /// # Arguments
    ///
    /// * `storage` - a mutable reference to the storage this item should go to
    /// * `key` - a byte slice representing the key to access the stored item
    /// * `value` - a reference to the item to store
    fn remove_impl(&self, storage: &mut dyn Storage, key: &[u8]) -> StdResult<()> {
        let prefixed_key = [self.as_slice(), key].concat();
        storage.remove(&prefixed_key);
        Ok(())
    }
}

/// An iterator over the contents of the Stack store.
pub struct StackIter<'a, T>
where
    T: Serialize + DeserializeOwned,
{
    stack: &'a Stack<'a, T>,
    storage: &'a dyn Storage,
    start: u32,
    end: u32,
}

impl<'a, T> StackIter<'a, T>
where
    T: Serialize + DeserializeOwned,
{
    /// constructor
    pub fn new(stack: &'a Stack<'a, T>, storage: &'a dyn Storage, start: u32, end: u32) -> Self {
        Self {
            stack,
            storage,
            start,
            end,
        }
    }
}

impl<'a, T> Iterator for StackIter<'a, T>
where
    T: Serialize + DeserializeOwned,
{
    type Item = StdResult<T>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.start >= self.end {
            return None;
        }
        let item = self.stack.get_at(self.storage, self.start).unwrap();
        self.start += 1;
        Some(Ok(item))
    }

    // This needs to be implemented correctly for `ExactSizeIterator` to work.
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = (self.end - self.start) as usize;
        (len, Some(len))
    }

    // `nth` is implemented manually, because it is used in the standard library whenever
    // it wants to skip over elements, but the default implementation repeatedly calls next.
    // because that is very expensive in this case, and the items are just discarded, we wan
    // do better here.
    // In practice, this enables cheap paging over the storage by calling:
    // `stack.iter().skip(start).take(length).collect()`
    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.start = self.start.saturating_add(n as u32);
        self.next()
    }
}

impl<'a, T> DoubleEndedIterator for StackIter<'a, T>
where
    T: Serialize + DeserializeOwned,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.start >= self.end {
            return None;
        }
        self.end -= 1;
        let item = self.stack.get_at(self.storage, self.end).unwrap();
        Some(Ok(item))
    }

    // I implement `nth_back` manually because it is used in the standard library whenever
    // it wants to skip over elements, but the default implementation repeatedly calls next_back.
    // because that is very expensive in this case, and the items are just discarded, we wan
    // do better here.
    // In practice, this enables cheap paging over the storage by calling:
    // `stack.iter().skip(start).take(length).collect()`
    fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
        self.end = self.end.saturating_sub(n as u32);
        self.next_back()
    }
}

// This enables writing `stack.iter().skip(n).rev()`
impl<'a, T> ExactSizeIterator for StackIter<'a, T> where T: Serialize + DeserializeOwned {}

#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::MockStorage;

    use super::*;

    #[test]
    fn test_push_pop() -> StdResult<()> {
        let mut storage = MockStorage::new();
        let stack: Stack<i32> = Stack::new("test");
        stack.push(&mut storage, &1234)?;
        stack.push(&mut storage, &2143)?;
        stack.push(&mut storage, &3412)?;
        stack.push(&mut storage, &4321)?;

        assert_eq!(stack.pop(&mut storage), Some(4321));
        assert_eq!(stack.pop(&mut storage), Some(3412));
        assert_eq!(stack.pop(&mut storage), Some(2143));
        assert_eq!(stack.pop(&mut storage), Some(1234));
        assert!(stack.pop(&mut storage).is_none());

        Ok(())
    }

    #[test]
    fn test_length() -> StdResult<()> {
        let mut storage = MockStorage::new();
        let stack: Stack<i32> = Stack::new("test");

        assert!(stack.length.eq(&None));
        assert_eq!(stack.get_len(&mut storage)?, 0);
        assert!(stack.length.eq(&None));

        stack.push(&mut storage, &1234)?;
        stack.push(&mut storage, &2143)?;
        stack.push(&mut storage, &3412)?;
        stack.push(&mut storage, &4321)?;
        // assert!(stack.length.eq(&Some(4)));
        assert_eq!(stack.get_len(&mut storage)?, 4);

        assert_eq!(stack.pop(&mut storage), Some(4321));
        assert_eq!(stack.pop(&mut storage), Some(3412));
        assert_eq!(stack.get_len(&mut storage)?, 2);

        assert_eq!(stack.pop(&mut storage), Some(2143));
        assert_eq!(stack.pop(&mut storage), Some(1234));
        assert_eq!(stack.get_len(&mut storage)?, 0);

        assert!(stack.pop(&mut storage).is_none());
        assert_eq!(stack.get_len(&mut storage)?, 0);

        Ok(())
    }

    #[test]
    fn test_iterator() -> StdResult<()> {
        let mut storage = MockStorage::new();
        let stack: Stack<i32> = Stack::new("test");
        stack.push(&mut storage, &1234)?;
        stack.push(&mut storage, &2143)?;
        stack.push(&mut storage, &3412)?;
        stack.push(&mut storage, &4321)?;

        // iterate twice to make sure nothing changed
        let mut iter = stack.iter(&storage)?;
        assert_eq!(iter.next(), Some(Ok(1234)));
        assert_eq!(iter.next(), Some(Ok(2143)));
        assert_eq!(iter.next(), Some(Ok(3412)));
        assert_eq!(iter.next(), Some(Ok(4321)));
        assert_eq!(iter.next(), None);

        let mut iter = stack.iter(&storage)?;
        assert_eq!(iter.next(), Some(Ok(1234)));
        assert_eq!(iter.next(), Some(Ok(2143)));
        assert_eq!(iter.next(), Some(Ok(3412)));
        assert_eq!(iter.next(), Some(Ok(4321)));
        assert_eq!(iter.next(), None);

        // make sure our implementation of `nth` doesn't break anything
        let mut iter = stack.iter(&storage)?.skip(2);
        assert_eq!(iter.next(), Some(Ok(3412)));
        assert_eq!(iter.next(), Some(Ok(4321)));
        assert_eq!(iter.next(), None);

        Ok(())
    }

    #[test]
    fn test_reverse_iterator() -> StdResult<()> {
        let mut storage = MockStorage::new();
        let stack: Stack<i32> = Stack::new("test");
        stack.push(&mut storage, &1234)?;
        stack.push(&mut storage, &2143)?;
        stack.push(&mut storage, &3412)?;
        stack.push(&mut storage, &4321)?;

        let mut iter = stack.iter(&storage)?.rev();
        assert_eq!(iter.next(), Some(Ok(4321)));
        assert_eq!(iter.next(), Some(Ok(3412)));
        assert_eq!(iter.next(), Some(Ok(2143)));
        assert_eq!(iter.next(), Some(Ok(1234)));
        assert_eq!(iter.next(), None);

        // iterate twice to make sure nothing changed
        let mut iter = stack.iter(&storage)?.rev();
        assert_eq!(iter.next(), Some(Ok(4321)));
        assert_eq!(iter.next(), Some(Ok(3412)));
        assert_eq!(iter.next(), Some(Ok(2143)));
        assert_eq!(iter.next(), Some(Ok(1234)));
        assert_eq!(iter.next(), None);

        // make sure our implementation of `nth_back` doesn't break anything
        let mut iter = stack.iter(&storage)?.rev().skip(2);
        assert_eq!(iter.next(), Some(Ok(2143)));
        assert_eq!(iter.next(), Some(Ok(1234)));
        assert_eq!(iter.next(), None);

        // make sure our implementation of `ExactSizeIterator` works well
        let mut iter = stack.iter(&storage)?.skip(2).rev();
        assert_eq!(iter.next(), Some(Ok(4321)));
        assert_eq!(iter.next(), Some(Ok(3412)));
        assert_eq!(iter.next(), None);

        Ok(())
    }

    #[test]
    fn test_json_push_pop() -> StdResult<()> {
        let mut storage = MockStorage::new();
        let stack: Stack<i32> = Stack::new("test");
        stack.push(&mut storage, &1234)?;
        stack.push(&mut storage, &2143)?;
        stack.push(&mut storage, &3412)?;
        stack.push(&mut storage, &4321)?;

        assert_eq!(stack.pop(&mut storage), Some(4321));
        assert_eq!(stack.pop(&mut storage), Some(3412));
        assert_eq!(stack.pop(&mut storage), Some(2143));
        assert_eq!(stack.pop(&mut storage), Some(1234));
        assert!(stack.pop(&mut storage).is_none());

        Ok(())
    }

    #[test]
    fn test_suffixed_pop() -> StdResult<()> {
        let mut storage = MockStorage::new();
        let suffix: &str = "test_suffix";
        let original_store: Stack<i32> = Stack::new("test");
        let stack = original_store.add_suffix(suffix);
        stack.push(&mut storage, &1234)?;
        stack.push(&mut storage, &2143)?;
        stack.push(&mut storage, &3412)?;
        stack.push(&mut storage, &4321)?;

        assert_eq!(stack.pop(&mut storage), Some(4321));
        assert_eq!(stack.pop(&mut storage), Some(3412));
        assert_eq!(stack.pop(&mut storage), Some(2143));
        assert_eq!(stack.pop(&mut storage), Some(1234));
        assert!(stack.pop(&mut storage).is_none());

        Ok(())
    }

    #[test]
    fn test_suffixed_reverse_iter() -> StdResult<()> {
        let mut storage = MockStorage::new();
        let suffix: &str = "test_suffix";
        let original_store: Stack<i32> = Stack::new("test");
        let stack = original_store.add_suffix(suffix);

        stack.push(&mut storage, &1234)?;
        stack.push(&mut storage, &2143)?;
        stack.push(&mut storage, &3412)?;
        stack.push(&mut storage, &4321)?;

        assert_eq!(original_store.get_len(&storage)?, 0);

        let mut iter = stack.iter(&storage)?.rev();
        assert_eq!(iter.next(), Some(Ok(4321)));
        assert_eq!(iter.next(), Some(Ok(3412)));
        assert_eq!(iter.next(), Some(Ok(2143)));
        assert_eq!(iter.next(), Some(Ok(1234)));
        assert_eq!(iter.next(), None);

        // iterate twice to make sure nothing changed
        let mut iter = stack.iter(&storage)?.rev();
        assert_eq!(iter.next(), Some(Ok(4321)));
        assert_eq!(iter.next(), Some(Ok(3412)));
        assert_eq!(iter.next(), Some(Ok(2143)));
        assert_eq!(iter.next(), Some(Ok(1234)));
        assert_eq!(iter.next(), None);

        // make sure our implementation of `nth_back` doesn't break anything
        let mut iter = stack.iter(&storage)?.rev().skip(2);
        assert_eq!(iter.next(), Some(Ok(2143)));
        assert_eq!(iter.next(), Some(Ok(1234)));
        assert_eq!(iter.next(), None);

        // make sure our implementation of `ExactSizeIterator` works well
        let mut iter = stack.iter(&storage)?.skip(2).rev();
        assert_eq!(iter.next(), Some(Ok(4321)));
        assert_eq!(iter.next(), Some(Ok(3412)));
        assert_eq!(iter.next(), None);

        Ok(())
    }

    #[test]
    fn test_suffix_iter() -> StdResult<()> {
        let mut storage = MockStorage::new();
        let suffix: &str = "test_suffix";
        let original_store: Stack<i32> = Stack::new("test");
        let stack = original_store.add_suffix(suffix);

        stack.push(&mut storage, &1234)?;
        stack.push(&mut storage, &2143)?;
        stack.push(&mut storage, &3412)?;
        stack.push(&mut storage, &4321)?;

        // iterate twice to make sure nothing changed
        let mut iter = stack.iter(&storage)?;
        assert_eq!(iter.next(), Some(Ok(1234)));
        assert_eq!(iter.next(), Some(Ok(2143)));
        assert_eq!(iter.next(), Some(Ok(3412)));
        assert_eq!(iter.next(), Some(Ok(4321)));
        assert_eq!(iter.next(), None);

        let mut iter = stack.iter(&storage)?;
        assert_eq!(iter.next(), Some(Ok(1234)));
        assert_eq!(iter.next(), Some(Ok(2143)));
        assert_eq!(iter.next(), Some(Ok(3412)));
        assert_eq!(iter.next(), Some(Ok(4321)));
        assert_eq!(iter.next(), None);

        // make sure our implementation of `nth` doesn't break anything
        let mut iter = stack.iter(&storage)?.skip(2);
        assert_eq!(iter.next(), Some(Ok(3412)));
        assert_eq!(iter.next(), Some(Ok(4321)));
        assert_eq!(iter.next(), None);

        Ok(())
    }

    #[test]
    fn test_removes() -> StdResult<()> {
        let mut storage = MockStorage::new();
        let queue: Stack<i32> = Stack::new("test");
        queue.push(&mut storage, &1)?;
        queue.push(&mut storage, &2)?;
        queue.push(&mut storage, &3)?;
        queue.push(&mut storage, &4)?;
        queue.push(&mut storage, &5)?;
        queue.push(&mut storage, &6)?;
        queue.push(&mut storage, &7)?;
        queue.push(&mut storage, &8)?;

        assert_eq!(queue.pop(&mut storage), Some(8));
        assert_eq!(queue.get_at(&storage, 6), Some(7));
        assert_eq!(queue.get_at(&storage, 5), Some(6));
        assert_eq!(queue.get_at(&storage, 4), Some(5));
        assert_eq!(queue.get_at(&storage, 3), Some(4));
        assert_eq!(queue.get_at(&storage, 2), Some(3));
        assert_eq!(queue.get_at(&storage, 1), Some(2));
        assert_eq!(queue.get_at(&storage, 0), Some(1));

        assert_eq!(queue.pop(&mut storage), Some(7));
        assert_eq!(queue.get_at(&storage, 5), Some(6));
        assert_eq!(queue.get_at(&storage, 4), Some(5));
        assert_eq!(queue.get_at(&storage, 3), Some(4));
        assert_eq!(queue.get_at(&storage, 2), Some(3));
        assert_eq!(queue.get_at(&storage, 1), Some(2));
        assert_eq!(queue.get_at(&storage, 0), Some(1));

        assert_eq!(queue.pop(&mut storage), Some(6));
        assert_eq!(queue.get_at(&storage, 4), Some(5));
        assert_eq!(queue.get_at(&storage, 3), Some(4));
        assert_eq!(queue.get_at(&storage, 2), Some(3));
        assert_eq!(queue.get_at(&storage, 1), Some(2));
        assert_eq!(queue.get_at(&storage, 0), Some(1));

        assert_eq!(queue.pop(&mut storage), Some(5));
        assert_eq!(queue.get_at(&storage, 3), Some(4));
        assert_eq!(queue.get_at(&storage, 2), Some(3));
        assert_eq!(queue.get_at(&storage, 1), Some(2));
        assert_eq!(queue.get_at(&storage, 0), Some(1));

        assert_eq!(queue.pop(&mut storage), Some(4));
        assert_eq!(queue.get_at(&storage, 2), Some(3));
        assert_eq!(queue.get_at(&storage, 1), Some(2));
        assert_eq!(queue.get_at(&storage, 0), Some(1));

        assert_eq!(queue.pop(&mut storage), Some(3));
        assert_eq!(queue.get_at(&storage, 1), Some(2));
        assert_eq!(queue.get_at(&storage, 0), Some(1));

        assert_eq!(queue.pop(&mut storage), Some(2));
        assert_eq!(queue.get_at(&storage, 0), Some(1));

        assert_eq!(queue.pop(&mut storage), Some(1));

        assert!(queue.pop(&mut storage).is_none());
        Ok(())
    }

    #[test]
    fn test_paging() -> StdResult<()> {
        let mut storage = MockStorage::new();
        let stack: Stack<u32> = Stack::new("test");

        let page_size: u32 = 5;
        let total_items: u32 = 50;

        for i in 0..total_items {
            stack.push(&mut storage, &i)?;
        }

        for i in 0..((total_items / page_size) - 1) {
            let start_page = i;

            let values = stack.paging(&storage, start_page, page_size)?;

            for (index, value) in values.iter().enumerate() {
                assert_eq!(value, &(page_size * start_page + index as u32))
            }
        }

        Ok(())
    }
}
