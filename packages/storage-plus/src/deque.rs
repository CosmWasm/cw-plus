use std::{any::type_name, convert::TryInto, marker::PhantomData};

use cosmwasm_std::{to_vec, StdError, StdResult, Storage};
use serde::{de::DeserializeOwned, Serialize};

use crate::helpers::{may_deserialize, namespaces_with_key};

// metadata keys need to have different length than the position type (4 bytes) to prevent collisions
const TAIL_KEY: &[u8] = b"t";
const HEAD_KEY: &[u8] = b"h";

/// A deque stores multiple items at the given key. It provides efficient FIFO and LIFO access.
pub struct Deque<'a, T> {
    // prefix of the deque items
    namespace: &'a [u8],
    // see https://doc.rust-lang.org/std/marker/struct.PhantomData.html#unused-type-parameters for why this is needed
    item_type: PhantomData<T>,
}

impl<'a, T> Deque<'a, T> {
    pub const fn new(prefix: &'a str) -> Self {
        Self {
            namespace: prefix.as_bytes(),
            item_type: PhantomData,
        }
    }
}

impl<'a, T: Serialize + DeserializeOwned> Deque<'a, T> {
    /// Adds the given value to the end of the deque
    pub fn push_back(&self, storage: &mut dyn Storage, value: &T) -> StdResult<()> {
        // save value
        let pos = self.tail(storage)?;
        self.set_unchecked(storage, pos, value)?;
        // update tail
        self.set_tail(storage, pos.wrapping_add(1));

        Ok(())
    }

    /// Adds the given value to the front of the deque
    pub fn push_front(&self, storage: &mut dyn Storage, value: &T) -> StdResult<()> {
        // need to subtract first, because head potentially points to existing element
        let pos = self.head(storage)?.wrapping_sub(1);
        self.set_unchecked(storage, pos, value)?;
        // update head
        self.set_head(storage, pos);

        Ok(())
    }

    /// Removes the last element of the deque and returns it
    pub fn pop_back(&self, storage: &mut dyn Storage) -> StdResult<Option<T>> {
        // get position
        let pos = self.tail(storage)?.wrapping_sub(1);
        let value = self.get_unchecked(storage, pos)?;
        if value.is_some() {
            self.remove_unchecked(storage, pos);
            // only update tail if a value was popped
            self.set_tail(storage, pos);
        }
        Ok(value)
    }

    /// Removes the first element of the deque and returns it
    pub fn pop_front(&self, storage: &mut dyn Storage) -> StdResult<Option<T>> {
        // get position
        let pos = self.head(storage)?;
        let value = self.get_unchecked(storage, pos)?;
        if value.is_some() {
            self.remove_unchecked(storage, pos);
            // only update head if a value was popped
            self.set_head(storage, pos.wrapping_add(1));
        }
        Ok(value)
    }

    /// Returns the first element of the deque without removing it
    pub fn front(&self, storage: &dyn Storage) -> StdResult<Option<T>> {
        let pos = self.head(storage)?;
        self.get_unchecked(storage, pos)
    }

    /// Returns the first element of the deque without removing it
    pub fn back(&self, storage: &dyn Storage) -> StdResult<Option<T>> {
        let pos = self.tail(storage)?.wrapping_sub(1);
        self.get_unchecked(storage, pos)
    }

    /// Gets the length of the deque.
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self, storage: &dyn Storage) -> StdResult<u32> {
        Ok(self.tail(storage)?.wrapping_sub(self.head(storage)?))
    }

    /// Returns `true` if the deque contains no elements.
    pub fn is_empty(&self, storage: &dyn Storage) -> StdResult<bool> {
        Ok(self.len(storage)? == 0)
    }

    /// Gets the head position from storage.
    ///
    /// Unless the deque is empty, this points to the first element.
    #[inline]
    fn head(&self, storage: &dyn Storage) -> StdResult<u32> {
        self.read_meta_key(storage, HEAD_KEY)
    }

    /// Gets the tail position from storage.
    ///
    /// This points to the first empty position after the last element.
    #[inline]
    fn tail(&self, storage: &dyn Storage) -> StdResult<u32> {
        self.read_meta_key(storage, TAIL_KEY)
    }

    #[inline]
    fn set_head(&self, storage: &mut dyn Storage, value: u32) {
        self.set_meta_key(storage, HEAD_KEY, value);
    }

    #[inline]
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

    /// Tries to get the value at the given position
    /// Used internally
    fn get_unchecked(&self, storage: &dyn Storage, pos: u32) -> StdResult<Option<T>> {
        let prefixed_key = namespaces_with_key(&[self.namespace], &pos.to_be_bytes());
        may_deserialize(&storage.get(&prefixed_key))
    }

    /// Removes the value at the given position
    /// Used internally
    fn remove_unchecked(&self, storage: &mut dyn Storage, pos: u32) {
        let prefixed_key = namespaces_with_key(&[self.namespace], &pos.to_be_bytes());
        storage.remove(&prefixed_key);
    }

    /// Tries to set the value at the given position
    /// Used internally when pushing
    fn set_unchecked(&self, storage: &mut dyn Storage, pos: u32, value: &T) -> StdResult<()> {
        let prefixed_key = namespaces_with_key(&[self.namespace], &pos.to_be_bytes());
        storage.set(&prefixed_key, &to_vec(value)?);

        Ok(())
    }
}

#[cfg(feature = "iterator")]
impl<'a, T: Serialize + DeserializeOwned> Deque<'a, T> {
    pub fn iter(&self, storage: &'a dyn Storage) -> StdResult<DequeIter<T>> {
        Ok(DequeIter {
            deque: self,
            storage,
            start: self.head(storage)?,
            end: self.tail(storage)?,
        })
    }
}

#[cfg(feature = "iterator")]
pub struct DequeIter<'a, T>
where
    T: Serialize + DeserializeOwned,
{
    deque: &'a Deque<'a, T>,
    storage: &'a dyn Storage,
    start: u32,
    end: u32,
}

#[cfg(feature = "iterator")]
impl<'a, T> Iterator for DequeIter<'a, T>
where
    T: Serialize + DeserializeOwned,
{
    type Item = StdResult<T>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.start == self.end {
            return None;
        }

        let item = self
            .deque
            .get_unchecked(self.storage, self.start)
            .and_then(|item| item.ok_or_else(|| StdError::not_found(type_name::<T>())));
        self.start = self.start.wrapping_add(1);

        Some(item)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.end.wrapping_sub(self.start) as usize;
        (len, Some(len))
    }

    // The default implementation calls `next` repeatedly, which is very costly in our case.
    // It is used when skipping over items, so this allows cheap skipping.
    //
    // Once `advance_by` is stabilized, we can implement that instead (`nth` calls it internally).
    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        // make sure that we don't skip past the end
        if self.end.wrapping_sub(self.start) < n as u32 {
            // mark as empty
            self.start = self.end;
        } else {
            self.start = self.start.wrapping_add(n as u32);
        }
        self.next()
    }
}

#[cfg(feature = "iterator")]
impl<'a, T> DoubleEndedIterator for DequeIter<'a, T>
where
    T: Serialize + DeserializeOwned,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.start == self.end {
            return None;
        }

        let item = self
            .deque
            .get_unchecked(self.storage, self.end.wrapping_sub(1)) // end points to position after last element
            .and_then(|item| item.ok_or_else(|| StdError::not_found(type_name::<T>())));
        self.end = self.end.wrapping_sub(1);

        Some(item)
    }

    // see [`DequeIter::nth`]
    fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
        // make sure that we don't skip past the start
        if self.end.wrapping_sub(self.start) < n as u32 {
            // mark as empty
            self.end = self.start;
        } else {
            self.end = self.end.wrapping_sub(n as u32);
        }
        self.next_back()
    }
}
#[cfg(test)]
mod tests {
    use crate::deque::Deque;

    use cosmwasm_std::testing::MockStorage;
    use cosmwasm_std::{StdError, StdResult};
    use serde::{Deserialize, Serialize};

    #[test]
    fn push_and_pop() {
        const PEOPLE: Deque<String> = Deque::new("people");
        let mut store = MockStorage::new();

        // push some entries
        PEOPLE.push_back(&mut store, &"jack".to_owned()).unwrap();
        PEOPLE.push_back(&mut store, &"john".to_owned()).unwrap();
        PEOPLE.push_back(&mut store, &"joanne".to_owned()).unwrap();

        // pop them, should be in correct order
        assert_eq!("jack", PEOPLE.pop_front(&mut store).unwrap().unwrap());
        assert_eq!("john", PEOPLE.pop_front(&mut store).unwrap().unwrap());

        // push again in-between
        PEOPLE.push_back(&mut store, &"jason".to_owned()).unwrap();

        // pop last person from first batch
        assert_eq!("joanne", PEOPLE.pop_front(&mut store).unwrap().unwrap());

        // pop the entry pushed in-between
        assert_eq!("jason", PEOPLE.pop_front(&mut store).unwrap().unwrap());

        // nothing after that
        assert_eq!(None, PEOPLE.pop_front(&mut store).unwrap());

        // now push to the front
        PEOPLE.push_front(&mut store, &"pascal".to_owned()).unwrap();
        PEOPLE.push_front(&mut store, &"peter".to_owned()).unwrap();
        PEOPLE.push_front(&mut store, &"paul".to_owned()).unwrap();

        assert_eq!("pascal", PEOPLE.pop_back(&mut store).unwrap().unwrap());
        assert_eq!("paul", PEOPLE.pop_front(&mut store).unwrap().unwrap());
        assert_eq!("peter", PEOPLE.pop_back(&mut store).unwrap().unwrap());
    }

    #[test]
    fn length() {
        let deque: Deque<u32> = Deque::new("test");
        let mut store = MockStorage::new();

        assert_eq!(deque.len(&store).unwrap(), 0);
        assert!(deque.is_empty(&store).unwrap());

        // push some entries
        deque.push_front(&mut store, &1234).unwrap();
        deque.push_back(&mut store, &2345).unwrap();
        deque.push_front(&mut store, &3456).unwrap();
        deque.push_back(&mut store, &4567).unwrap();
        assert_eq!(deque.len(&store).unwrap(), 4);
        assert!(!deque.is_empty(&store).unwrap());

        // pop some
        deque.pop_front(&mut store).unwrap();
        deque.pop_back(&mut store).unwrap();
        deque.pop_front(&mut store).unwrap();
        assert_eq!(deque.len(&store).unwrap(), 1);
        assert!(!deque.is_empty(&store).unwrap());

        // pop the last one
        deque.pop_front(&mut store).unwrap();
        assert_eq!(deque.len(&store).unwrap(), 0);
        assert!(deque.is_empty(&store).unwrap());

        // should stay 0 after that
        assert_eq!(deque.pop_back(&mut store).unwrap(), None);
        assert_eq!(
            deque.len(&store).unwrap(),
            0,
            "popping from empty deque should keep length 0"
        );
        assert!(deque.is_empty(&store).unwrap());
    }

    #[test]
    #[cfg(feature = "iterator")]
    fn iterator() {
        let deque: Deque<u32> = Deque::new("test");
        let mut store = MockStorage::new();

        // push some items
        deque.push_back(&mut store, &1).unwrap();
        deque.push_back(&mut store, &2).unwrap();
        deque.push_back(&mut store, &3).unwrap();
        deque.push_back(&mut store, &4).unwrap();

        let items: StdResult<Vec<_>> = deque.iter(&store).unwrap().collect();
        assert_eq!(items.unwrap(), [1, 2, 3, 4]);

        // nth should work correctly
        let mut iter = deque.iter(&store).unwrap();
        assert_eq!(iter.nth(6), None);
        assert_eq!(iter.start, iter.end, "iter should detect skipping too far");
        assert_eq!(iter.next(), None);

        let mut iter = deque.iter(&store).unwrap();
        assert_eq!(iter.nth(1).unwrap().unwrap(), 2);
        assert_eq!(iter.next().unwrap().unwrap(), 3);
    }

    #[test]
    #[cfg(feature = "iterator")]
    fn reverse_iterator() {
        let deque: Deque<u32> = Deque::new("test");
        let mut store = MockStorage::new();

        // push some items
        deque.push_back(&mut store, &1).unwrap();
        deque.push_back(&mut store, &2).unwrap();
        deque.push_back(&mut store, &3).unwrap();
        deque.push_back(&mut store, &4).unwrap();

        let items: StdResult<Vec<_>> = deque.iter(&store).unwrap().rev().collect();
        assert_eq!(items.unwrap(), [4, 3, 2, 1]);

        // nth should work correctly
        let mut iter = deque.iter(&store).unwrap();
        assert_eq!(iter.nth_back(6), None);
        assert_eq!(iter.start, iter.end, "iter should detect skipping too far");
        assert_eq!(iter.next_back(), None);

        let mut iter = deque.iter(&store).unwrap().rev();
        assert_eq!(iter.nth(1).unwrap().unwrap(), 3);
        assert_eq!(iter.next().unwrap().unwrap(), 2);

        // mixed
        let mut iter = deque.iter(&store).unwrap();
        assert_eq!(iter.next().unwrap().unwrap(), 1);
        assert_eq!(iter.next_back().unwrap().unwrap(), 4);
        assert_eq!(iter.next_back().unwrap().unwrap(), 3);
        assert_eq!(iter.next().unwrap().unwrap(), 2);
        assert_eq!(iter.next(), None);
        assert_eq!(iter.next_back(), None);
    }

    #[test]
    fn wrapping() {
        let deque: Deque<u32> = Deque::new("test");
        let mut store = MockStorage::new();

        // simulate deque that was pushed and popped `u32::MAX` times
        deque.set_head(&mut store, u32::MAX);
        deque.set_tail(&mut store, u32::MAX);

        // should be empty
        assert_eq!(deque.pop_front(&mut store).unwrap(), None);
        assert_eq!(deque.len(&store).unwrap(), 0);

        // pushing should still work
        deque.push_back(&mut store, &1).unwrap();
        assert_eq!(
            deque.len(&store).unwrap(),
            1,
            "length should calculate correctly, even when wrapping"
        );
        assert_eq!(
            deque.pop_front(&mut store).unwrap(),
            Some(1),
            "popping should work, even when wrapping"
        );
    }

    #[test]
    #[cfg(feature = "iterator")]
    fn wrapping_iterator() {
        let deque: Deque<u32> = Deque::new("test");
        let mut store = MockStorage::new();

        deque.set_head(&mut store, u32::MAX);
        deque.set_tail(&mut store, u32::MAX);

        deque.push_back(&mut store, &1).unwrap();
        deque.push_back(&mut store, &2).unwrap();
        deque.push_back(&mut store, &3).unwrap();
        deque.push_back(&mut store, &4).unwrap();
        deque.push_back(&mut store, &5).unwrap();

        let mut iter = deque.iter(&store).unwrap();
        assert_eq!(iter.next().unwrap().unwrap(), 1);
        assert_eq!(iter.next().unwrap().unwrap(), 2);
        assert_eq!(iter.next_back().unwrap().unwrap(), 5);
        assert_eq!(iter.nth(1).unwrap().unwrap(), 4);
        assert_eq!(iter.nth(1), None);
        assert_eq!(iter.start, iter.end);
    }

    #[test]
    fn front_back() {
        let deque: Deque<u64> = Deque::new("test");
        let mut store = MockStorage::new();

        assert_eq!(deque.back(&store).unwrap(), None);
        deque.push_back(&mut store, &1).unwrap();
        assert_eq!(deque.back(&store).unwrap(), Some(1));
        assert_eq!(deque.front(&store).unwrap(), Some(1));
        deque.push_back(&mut store, &2).unwrap();
        assert_eq!(deque.back(&store).unwrap(), Some(2));
        assert_eq!(deque.front(&store).unwrap(), Some(1));
        deque.push_front(&mut store, &3).unwrap();
        assert_eq!(deque.back(&store).unwrap(), Some(2));
        assert_eq!(deque.front(&store).unwrap(), Some(3));
    }

    #[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
    struct Data {
        pub name: String,
        pub age: i32,
    }

    const DATA: Deque<Data> = Deque::new("data");

    #[test]
    #[cfg(feature = "iterator")]
    fn readme_works() -> StdResult<()> {
        let mut store = MockStorage::new();

        // read methods return a wrapped Option<T>, so None if the deque is empty
        let empty = DATA.front(&store)?;
        assert_eq!(None, empty);

        // some example entries
        let p1 = Data {
            name: "admin".to_string(),
            age: 1234,
        };
        let p2 = Data {
            name: "user".to_string(),
            age: 123,
        };

        // use it like a queue by pushing and popping at opposite ends
        DATA.push_back(&mut store, &p1)?;
        DATA.push_back(&mut store, &p2)?;

        let admin = DATA.pop_front(&mut store)?;
        assert_eq!(admin.as_ref(), Some(&p1));
        let user = DATA.pop_front(&mut store)?;
        assert_eq!(user.as_ref(), Some(&p2));

        // or push and pop at the same end to use it as a stack
        DATA.push_back(&mut store, &p1)?;
        DATA.push_back(&mut store, &p2)?;

        let user = DATA.pop_back(&mut store)?;
        assert_eq!(user.as_ref(), Some(&p2));
        let admin = DATA.pop_back(&mut store)?;
        assert_eq!(admin.as_ref(), Some(&p1));

        // you can also iterate over it
        DATA.push_front(&mut store, &p1)?;
        DATA.push_front(&mut store, &p2)?;

        let all: StdResult<Vec<_>> = DATA.iter(&store)?.collect();
        assert_eq!(all?, [p2, p1]);

        Ok(())
    }

    #[test]
    #[cfg(feature = "iterator")]
    fn iterator_errors_when_item_missing() {
        let mut store = MockStorage::new();

        let deque = Deque::new("error_test");

        deque.push_back(&mut store, &1u32).unwrap();
        // manually remove it
        deque.remove_unchecked(&mut store, 0);

        let mut iter = deque.iter(&store).unwrap();

        assert!(
            matches!(iter.next(), Some(Err(StdError::NotFound { .. }))),
            "iterator should error when item is missing"
        );

        let mut iter = deque.iter(&store).unwrap().rev();

        assert!(
            matches!(iter.next(), Some(Err(StdError::NotFound { .. }))),
            "reverse iterator should error when item is missing"
        );
    }
}
