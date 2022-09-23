use std::{convert::TryInto, marker::PhantomData};

use cosmwasm_std::{to_vec, StdError, StdResult, Storage};
use serde::{de::DeserializeOwned, Serialize};

use crate::helpers::{may_deserialize, namespaces_with_key};

// metadata keys need to have different length than the position type (4 bytes) to prevent collisions
const TAIL_KEY: &[u8] = b"t";
const HEAD_KEY: &[u8] = b"h";

/// A queue stores multiple items at the given key. It provides efficient FIFO access.
pub struct Queue<'a, T> {
    // prefix of the queue items
    namespace: &'a [u8],
    // see https://doc.rust-lang.org/std/marker/struct.PhantomData.html#unused-type-parameters for why this is needed
    item_type: PhantomData<T>,
}

impl<'a, T> Queue<'a, T> {
    pub const fn new(prefix: &'a str) -> Self {
        Self {
            namespace: prefix.as_bytes(),
            item_type: PhantomData,
        }
    }
}

impl<'a, T: Serialize + DeserializeOwned> Queue<'a, T> {
    /// Adds the given value to the end of the queue
    pub fn push(&self, storage: &mut dyn Storage, value: &T) -> StdResult<()> {
        // save value
        let pos = self.tail(storage)?;
        self.set_at(storage, pos, value)?;
        // update tail
        self.set_tail(storage, pos.wrapping_add(1));

        Ok(())
    }

    /// Removes the first item of the queue and returns it
    pub fn pop(&self, storage: &mut dyn Storage) -> StdResult<Option<T>> {
        // get position
        let pos = self.head(storage)?;
        let value = self.get_at(storage, pos)?;
        if value.is_some() {
            self.remove_at(storage, pos);
            // only update head if a value was popped
            self.set_head(storage, pos.wrapping_add(1));
        }
        Ok(value)
    }

    /// Gets the length of the queue.
    pub fn len(&self, storage: &dyn Storage) -> StdResult<u32> {
        Ok(self.tail(storage)?.wrapping_sub(self.head(storage)?))
    }

    /// Returns `true` if the queue contains no elements.
    pub fn is_empty(&self, storage: &dyn Storage) -> StdResult<bool> {
        Ok(self.len(storage)? == 0)
    }

    /// Gets the head position from storage.
    ///
    /// Points to the front of the queue (where elements are popped).
    fn head(&self, storage: &dyn Storage) -> StdResult<u32> {
        self.read_meta_key(storage, HEAD_KEY)
    }

    /// Gets the tail position from storage.
    ///
    /// Points to the end of the queue (where elements are pushed).
    fn tail(&self, storage: &dyn Storage) -> StdResult<u32> {
        self.read_meta_key(storage, TAIL_KEY)
    }

    fn set_head(&self, storage: &mut dyn Storage, value: u32) {
        self.set_meta_key(storage, HEAD_KEY, value);
    }

    fn set_tail(&self, storage: &mut dyn Storage, value: u32) {
        self.set_meta_key(storage, TAIL_KEY, value);
    }

    /// Helper method for `tail` and `head` methods to handle reading the value from storage
    fn read_meta_key(&self, storage: &dyn Storage, key: &[u8]) -> StdResult<u32> {
        let full_key = namespaces_with_key(&[self.namespace], key);
        storage
            .get(&full_key)
            .map(|vec| {
                Ok(u32::from_be_bytes(
                    vec.as_slice()
                        .try_into()
                        .map_err(|e| StdError::parse_err("u32", e))?,
                ))
            })
            .unwrap_or(Ok(0))
    }

    /// Helper method for `set_tail` and `set_head` methods to write to storage
    #[inline]
    fn set_meta_key(&self, storage: &mut dyn Storage, key: &[u8], value: u32) {
        let full_key = namespaces_with_key(&[self.namespace], key);
        storage.set(&full_key, &value.to_be_bytes());
    }

    /// Tries to get the value at the given position (without bounds checking)
    /// Used internally when popping
    fn get_at(&self, storage: &dyn Storage, pos: u32) -> StdResult<Option<T>> {
        let prefixed_key = namespaces_with_key(&[self.namespace], &pos.to_be_bytes());
        may_deserialize(&storage.get(&prefixed_key))
    }

    /// Removes the value at the given position
    /// Used internally when popping
    fn remove_at(&self, storage: &mut dyn Storage, pos: u32) {
        let prefixed_key = namespaces_with_key(&[self.namespace], &pos.to_be_bytes());
        storage.remove(&prefixed_key);
    }

    /// Tries to set the value at the given position (without bounds checking)
    /// Used internally when pushing
    fn set_at(&self, storage: &mut dyn Storage, pos: u32, value: &T) -> StdResult<()> {
        let prefixed_key = namespaces_with_key(&[self.namespace], &pos.to_be_bytes());
        storage.set(&prefixed_key, &to_vec(value)?);

        Ok(())
    }
}

#[cfg(feature = "iterator")]
impl<'a, T: Serialize + DeserializeOwned> Queue<'a, T> {
    pub fn iter(&self, storage: &'a dyn Storage) -> StdResult<QueueIter<T>> {
        Ok(QueueIter {
            queue: self,
            storage,
            start: self.head(storage)?,
            end: self.tail(storage)?,
        })
    }
}

#[cfg(feature = "iterator")]
pub struct QueueIter<'a, T>
where
    T: Serialize + DeserializeOwned,
{
    queue: &'a Queue<'a, T>,
    storage: &'a dyn Storage,
    start: u32,
    end: u32,
}

impl<'a, T> Iterator for QueueIter<'a, T>
where
    T: Serialize + DeserializeOwned,
{
    type Item = StdResult<T>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.start == self.end {
            return None;
        }

        let item = self.queue.get_at(self.storage, self.start).transpose()?;
        self.start = self.start.wrapping_add(1);

        Some(item)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.end.wrapping_sub(self.start) as usize;
        (len, Some(len))
    }

    /// The default implementation calls `next` repeatedly, which is very costly in our case.
    /// It is used when skipping over items, so this allows cheap skipping.
    ///
    /// Once `advance_by` is stabilized, we can implement that instead (`nth` calls it internally).
    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        let start_lt_end = self.start < self.end;
        self.start = self.start.wrapping_add(n as u32);
        // make sure that we didn't skip past the end
        if self.start > self.end && start_lt_end || self.start < self.end && !start_lt_end {
            // start and end switched places, which means the iterator is empty now
            self.start = self.end;
        }
        self.next()
    }
}

#[cfg(test)]
mod tests {
    use crate::queue::Queue;

    use cosmwasm_std::testing::MockStorage;
    use cosmwasm_std::StdResult;

    #[test]
    fn push_and_pop() {
        const PEOPLE: Queue<String> = Queue::new("people");
        let mut store = MockStorage::new();

        // push some entries
        PEOPLE.push(&mut store, &"jack".to_owned()).unwrap();
        PEOPLE.push(&mut store, &"john".to_owned()).unwrap();
        PEOPLE.push(&mut store, &"joanne".to_owned()).unwrap();

        // pop them, should be in correct order
        assert_eq!("jack", PEOPLE.pop(&mut store).unwrap().unwrap());
        assert_eq!("john", PEOPLE.pop(&mut store).unwrap().unwrap());

        // push again in-between
        PEOPLE.push(&mut store, &"jason".to_owned()).unwrap();

        // pop last person from first batch
        assert_eq!("joanne", PEOPLE.pop(&mut store).unwrap().unwrap());

        // pop the entry pushed in-between
        assert_eq!("jason", PEOPLE.pop(&mut store).unwrap().unwrap());

        // nothing after that
        assert_eq!(None, PEOPLE.pop(&mut store).unwrap());
    }

    #[test]
    fn length() {
        let queue: Queue<u32> = Queue::new("test");
        let mut store = MockStorage::new();

        assert_eq!(queue.len(&store).unwrap(), 0);

        // push some entries
        queue.push(&mut store, &1234).unwrap();
        queue.push(&mut store, &2345).unwrap();
        queue.push(&mut store, &3456).unwrap();
        queue.push(&mut store, &4567).unwrap();
        assert_eq!(queue.len(&store).unwrap(), 4);

        // pop some
        queue.pop(&mut store).unwrap();
        queue.pop(&mut store).unwrap();
        queue.pop(&mut store).unwrap();
        assert_eq!(queue.len(&store).unwrap(), 1);

        // pop the last one
        queue.pop(&mut store).unwrap();
        assert_eq!(queue.len(&store).unwrap(), 0);

        // should stay 0 after that
        queue.pop(&mut store).unwrap();
        assert_eq!(
            queue.len(&store).unwrap(),
            0,
            "popping from empty queue should keep length 0"
        );
    }

    #[test]
    fn iterator() {
        let queue: Queue<u32> = Queue::new("test");
        let mut store = MockStorage::new();

        // push some items
        queue.push(&mut store, &1).unwrap();
        queue.push(&mut store, &2).unwrap();
        queue.push(&mut store, &3).unwrap();
        queue.push(&mut store, &4).unwrap();

        let items: StdResult<Vec<_>> = queue.iter(&mut store).unwrap().collect();
        assert_eq!(items.unwrap(), [1, 2, 3, 4]);

        let mut iter = queue.iter(&mut store).unwrap();
        assert_eq!(iter.nth(6), None);
        assert_eq!(iter.start, iter.end, "iter should detect skipping too far");
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn wrapping() {
        let queue: Queue<u32> = Queue::new("test");
        let mut store = MockStorage::new();

        // simulate queue that was pushed and popped `u32::MAX` times
        queue.set_head(&mut store, u32::MAX);
        queue.set_tail(&mut store, u32::MAX);

        // should be empty
        assert_eq!(queue.pop(&mut store).unwrap(), None);
        assert_eq!(queue.len(&store).unwrap(), 0);

        // pushing should still work
        queue.push(&mut store, &1).unwrap();
        assert_eq!(
            queue.len(&store).unwrap(),
            1,
            "length should calculate correctly, even when wrapping"
        );
        assert_eq!(
            queue.pop(&mut store).unwrap(),
            Some(1),
            "popping should work, even when wrapping"
        );
    }
}
