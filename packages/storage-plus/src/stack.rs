//! An "Stack" is a storage wrapper that guarantees constant-cost appending to and popping
//! from a list of items in storage.
//!
//! This is achieved by storing each item in a separate storage entry. A special key is reserved
//! for storing the length of the collection so far.
use std::any::type_name;
use std::convert::TryInto;
use std::marker::PhantomData;
use std::sync::Mutex;

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
    length: Mutex<Option<u32>>,
    item_type: PhantomData<T>,
}

impl<'a, T: Serialize + DeserializeOwned> Stack<'a, T> {
    /// constructor
    pub const fn new(prefix: &'a str) -> Self {
        Self {
            namespace: prefix.as_bytes(),
            prefix: None,
            length: Mutex::new(None),
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
            length: Mutex::new(None),
            item_type: self.item_type,
        }
    }
}

impl<'a, T: Serialize + DeserializeOwned> Stack<'a, T> {
    /// gets the length from storage, and otherwise sets it to 0
    pub fn get_len(&self, storage: &dyn Storage) -> StdResult<u32> {
        let mut may_len = self.length.lock().unwrap();
        match *may_len {
            Some(len) => Ok(len),
            None => {
                let len_key = [self.as_slice(), LEN_KEY].concat();
                if let Some(len_vec) = storage.get(&len_key) {
                    let len_bytes = len_vec
                        .as_slice()
                        .try_into()
                        .map_err(|err| StdError::parse_err("u32", err))?;
                    let len = u32::from_be_bytes(len_bytes);
                    *may_len = Some(len);
                    Ok(len)
                } else {
                    *may_len = Some(0);
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
    pub fn get_at(&self, storage: &dyn Storage, pos: u32) -> StdResult<T> {
        let len = self.get_len(storage)?;
        if pos > len {
            return Err(StdError::generic_err("Stack access out of bounds"));
        }
        self.get_at_unchecked(storage, pos)
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

        let mut may_len = self.length.lock().unwrap();
        *may_len = Some(len);
    }
    /// Clear the collection
    pub fn clear(&self, storage: &mut dyn Storage) {
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
    /// Pushes an item to Stack
    pub fn push(&self, storage: &mut dyn Storage, item: &T) -> StdResult<()> {
        let len = self.get_len(storage)?;
        self.set_at_unchecked(storage, len, item)?;
        self.set_len(storage, len + 1);
        Ok(())
    }
    /// Pops an item from Stack
    pub fn pop(&self, storage: &mut dyn Storage) -> StdResult<T> {
        if let Some(len) = self.get_len(storage)?.checked_sub(1) {
            let item = self.get_at_unchecked(storage, len);
            self.set_len(storage, len);
            item
        } else {
            Err(StdError::generic_err("Can not pop from empty Stack"))
        }
    }
    /// Remove an element from the collection at the specified position.
    ///
    /// Removing the last element has a constant cost.
    /// The cost of removing from the middle/start will depend on the proximity to tail of the list.
    /// All elements above the specified position will be shifted in storage.
    ///
    /// Removing an element from the start (head) of the collection
    /// has the worst runtime and gas cost.
    pub fn remove(&self, storage: &mut dyn Storage, pos: u32) -> StdResult<T> {
        let len = self.get_len(storage)?;

        if pos >= len {
            return Err(StdError::generic_err("DequeStorage access out of bounds"));
        }
        let item = self.get_at_unchecked(storage, pos);

        for i in pos..(len - 1) {
            let element_to_shift = self.get_at_unchecked(storage, i + 1)?;
            self.set_at_unchecked(storage, i, &element_to_shift)?;
        }
        self.set_len(storage, len - 1);
        item
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
            length: Mutex::new(None),
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
        let item = self.stack.get_at(self.storage, self.start);
        self.start += 1;
        Some(item)
    }

    // This needs to be implemented correctly for `ExactSizeIterator` to work.
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = (self.end - self.start) as usize;
        (len, Some(len))
    }

    // I implement `nth` manually because it is used in the standard library whenever
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
        let item = self.stack.get_at(self.storage, self.end);
        Some(item)
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

        assert_eq!(stack.pop(&mut storage), Ok(4321));
        assert_eq!(stack.pop(&mut storage), Ok(3412));
        assert_eq!(stack.pop(&mut storage), Ok(2143));
        assert_eq!(stack.pop(&mut storage), Ok(1234));
        assert!(stack.pop(&mut storage).is_err());

        Ok(())
    }

    #[test]
    fn test_length() -> StdResult<()> {
        let mut storage = MockStorage::new();
        let stack: Stack<i32> = Stack::new("test");

        assert!(stack.length.lock().unwrap().eq(&None));
        assert_eq!(stack.get_len(&mut storage)?, 0);
        assert!(stack.length.lock().unwrap().eq(&Some(0)));

        stack.push(&mut storage, &1234)?;
        stack.push(&mut storage, &2143)?;
        stack.push(&mut storage, &3412)?;
        stack.push(&mut storage, &4321)?;
        assert!(stack.length.lock().unwrap().eq(&Some(4)));
        assert_eq!(stack.get_len(&mut storage)?, 4);

        assert_eq!(stack.pop(&mut storage), Ok(4321));
        assert_eq!(stack.pop(&mut storage), Ok(3412));
        assert!(stack.length.lock().unwrap().eq(&Some(2)));
        assert_eq!(stack.get_len(&mut storage)?, 2);

        assert_eq!(stack.pop(&mut storage), Ok(2143));
        assert_eq!(stack.pop(&mut storage), Ok(1234));
        assert!(stack.length.lock().unwrap().eq(&Some(0)));
        assert_eq!(stack.get_len(&mut storage)?, 0);

        assert!(stack.pop(&mut storage).is_err());
        assert!(stack.length.lock().unwrap().eq(&Some(0)));
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

        assert_eq!(stack.pop(&mut storage), Ok(4321));
        assert_eq!(stack.pop(&mut storage), Ok(3412));
        assert_eq!(stack.pop(&mut storage), Ok(2143));
        assert_eq!(stack.pop(&mut storage), Ok(1234));
        assert!(stack.pop(&mut storage).is_err());

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

        assert_eq!(stack.pop(&mut storage), Ok(4321));
        assert_eq!(stack.pop(&mut storage), Ok(3412));
        assert_eq!(stack.pop(&mut storage), Ok(2143));
        assert_eq!(stack.pop(&mut storage), Ok(1234));
        assert!(stack.pop(&mut storage).is_err());

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
        let deque_store: Stack<i32> = Stack::new("test");
        deque_store.push(&mut storage, &1)?;
        deque_store.push(&mut storage, &2)?;
        deque_store.push(&mut storage, &3)?;
        deque_store.push(&mut storage, &4)?;
        deque_store.push(&mut storage, &5)?;
        deque_store.push(&mut storage, &6)?;
        deque_store.push(&mut storage, &7)?;
        deque_store.push(&mut storage, &8)?;

        assert!(deque_store.remove(&mut storage, 8).is_err());
        assert!(deque_store.remove(&mut storage, 9).is_err());

        assert_eq!(deque_store.remove(&mut storage, 7), Ok(8));
        assert_eq!(deque_store.get_at(&storage, 6), Ok(7));
        assert_eq!(deque_store.get_at(&storage, 5), Ok(6));
        assert_eq!(deque_store.get_at(&storage, 4), Ok(5));
        assert_eq!(deque_store.get_at(&storage, 3), Ok(4));
        assert_eq!(deque_store.get_at(&storage, 2), Ok(3));
        assert_eq!(deque_store.get_at(&storage, 1), Ok(2));
        assert_eq!(deque_store.get_at(&storage, 0), Ok(1));

        assert_eq!(deque_store.remove(&mut storage, 6), Ok(7));
        assert_eq!(deque_store.get_at(&storage, 5), Ok(6));
        assert_eq!(deque_store.get_at(&storage, 4), Ok(5));
        assert_eq!(deque_store.get_at(&storage, 3), Ok(4));
        assert_eq!(deque_store.get_at(&storage, 2), Ok(3));
        assert_eq!(deque_store.get_at(&storage, 1), Ok(2));
        assert_eq!(deque_store.get_at(&storage, 0), Ok(1));

        assert_eq!(deque_store.remove(&mut storage, 3), Ok(4));
        assert_eq!(deque_store.get_at(&storage, 4), Ok(6));
        assert_eq!(deque_store.get_at(&storage, 3), Ok(5));
        assert_eq!(deque_store.get_at(&storage, 2), Ok(3));
        assert_eq!(deque_store.get_at(&storage, 1), Ok(2));
        assert_eq!(deque_store.get_at(&storage, 0), Ok(1));

        assert_eq!(deque_store.remove(&mut storage, 1), Ok(2));
        assert_eq!(deque_store.get_at(&storage, 3), Ok(6));
        assert_eq!(deque_store.get_at(&storage, 2), Ok(5));
        assert_eq!(deque_store.get_at(&storage, 1), Ok(3));
        assert_eq!(deque_store.get_at(&storage, 0), Ok(1));

        assert_eq!(deque_store.remove(&mut storage, 2), Ok(5));
        assert_eq!(deque_store.get_at(&storage, 2), Ok(6));
        assert_eq!(deque_store.get_at(&storage, 1), Ok(3));
        assert_eq!(deque_store.get_at(&storage, 0), Ok(1));

        assert_eq!(deque_store.remove(&mut storage, 1), Ok(3));
        assert_eq!(deque_store.get_at(&storage, 1), Ok(6));
        assert_eq!(deque_store.get_at(&storage, 0), Ok(1));

        assert_eq!(deque_store.remove(&mut storage, 1), Ok(6));
        assert_eq!(deque_store.get_at(&storage, 0), Ok(1));

        assert_eq!(deque_store.remove(&mut storage, 0), Ok(1));

        assert!(deque_store.remove(&mut storage, 0).is_err());
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
