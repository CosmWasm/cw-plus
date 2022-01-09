use serde::de::DeserializeOwned;
use serde::Serialize;

use cosmwasm_std::{StdError, StdResult, Storage};

use crate::snapshot::{ChangeSet, Snapshot};
use crate::{Item, Map, Strategy};

/// Item that maintains a snapshot of one or more checkpoints.
/// We can query historical data as well as current state.
/// What data is snapshotted depends on the Strategy.
pub struct SnapshotItem<'a, T> {
    primary: Item<'a, T>,
    changelog_namespace: &'a str,
    snapshots: Snapshot<'a, (), T>,
}

impl<'a, T> SnapshotItem<'a, T> {
    /// Example:
    ///
    /// ```rust
    /// use cw_storage_plus::{SnapshotItem, Strategy};
    ///
    /// SnapshotItem::<'static, u64>::new(
    ///     "every",
    ///     "every__check",
    ///     "every__change",
    ///     Strategy::EveryBlock);
    /// ```
    pub const fn new(
        storage_key: &'a str,
        checkpoints: &'a str,
        changelog: &'a str,
        strategy: Strategy,
    ) -> Self {
        SnapshotItem {
            primary: Item::new(storage_key),
            changelog_namespace: changelog,
            snapshots: Snapshot::new(checkpoints, changelog, strategy),
        }
    }

    pub fn add_checkpoint(&self, store: &mut dyn Storage, height: u64) -> StdResult<()> {
        self.snapshots.add_checkpoint(store, height)
    }

    pub fn remove_checkpoint(&self, store: &mut dyn Storage, height: u64) -> StdResult<()> {
        self.snapshots.remove_checkpoint(store, height)
    }

    pub fn changelog(&self) -> Map<u64, ChangeSet<T>> {
        // Build and return a compatible Map with the proper key type
        Map::new(self.changelog_namespace)
    }
}

impl<'a, T> SnapshotItem<'a, T>
where
    T: Serialize + DeserializeOwned + Clone,
{
    /// load old value and store changelog
    fn write_change(&self, store: &mut dyn Storage, height: u64) -> StdResult<()> {
        // if there is already data in the changelog for this block, do not write more
        if self.snapshots.has_changelog(store, (), height)? {
            return Ok(());
        }
        // otherwise, store the previous value
        let old = self.primary.may_load(store)?;
        self.snapshots.write_changelog(store, (), height, old)
    }

    pub fn save(&self, store: &mut dyn Storage, data: &T, height: u64) -> StdResult<()> {
        if self.snapshots.should_checkpoint(store, &())? {
            self.write_change(store, height)?;
        }
        self.primary.save(store, data)
    }

    pub fn remove(&self, store: &mut dyn Storage, height: u64) -> StdResult<()> {
        if self.snapshots.should_checkpoint(store, &())? {
            self.write_change(store, height)?;
        }
        self.primary.remove(store);
        Ok(())
    }

    /// load will return an error if no data is set, or on parse error
    pub fn load(&self, store: &dyn Storage) -> StdResult<T> {
        self.primary.load(store)
    }

    /// may_load will parse the data stored if present, returns Ok(None) if no data there.
    /// returns an error on parsing issues
    pub fn may_load(&self, store: &dyn Storage) -> StdResult<Option<T>> {
        self.primary.may_load(store)
    }

    pub fn may_load_at_height(&self, store: &dyn Storage, height: u64) -> StdResult<Option<T>> {
        let snapshot = self.snapshots.may_load_at_height(store, (), height)?;

        if let Some(r) = snapshot {
            Ok(r)
        } else {
            // otherwise, return current value
            self.may_load(store)
        }
    }

    // If there is no checkpoint for that height, then we return StdError::NotFound
    pub fn assert_checkpointed(&self, store: &dyn Storage, height: u64) -> StdResult<()> {
        self.snapshots.assert_checkpointed(store, height)
    }

    /// Loads the data, perform the specified action, and store the result in the database.
    /// This is a shorthand for some common sequences, which may be useful.
    ///
    /// If the data exists, `action(Some(value))` is called. Otherwise `action(None)` is called.
    ///
    /// This is a bit more customized than needed to only read "old" value 1 time, not 2 per naive approach
    pub fn update<A, E>(&self, store: &mut dyn Storage, height: u64, action: A) -> Result<T, E>
    where
        A: FnOnce(Option<T>) -> Result<T, E>,
        E: From<StdError>,
    {
        let input = self.may_load(store)?;
        let output = action(input)?;
        self.save(store, &output, height)?;
        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bound::Bound;
    use cosmwasm_std::testing::MockStorage;

    type TestItem = SnapshotItem<'static, u64>;

    const NEVER: TestItem =
        SnapshotItem::new("never", "never__check", "never__change", Strategy::Never);
    const EVERY: TestItem = SnapshotItem::new(
        "every",
        "every__check",
        "every__change",
        Strategy::EveryBlock,
    );
    const SELECT: TestItem = SnapshotItem::new(
        "select",
        "select__check",
        "select__change",
        Strategy::Selected,
    );

    // Fills an item (u64) with the following writes:
    // 1: 5
    // 2: 7
    // 3: 8
    // 4: 1
    // 5: None
    // 6: 13
    // 7: None
    // 8: 22
    // Final value: 22
    // Value at beginning of 3 -> 7
    // Value at beginning of 5 -> 1
    fn init_data(item: &TestItem, storage: &mut dyn Storage) {
        item.save(storage, &5, 1).unwrap();
        item.save(storage, &7, 2).unwrap();

        // checkpoint 3
        item.add_checkpoint(storage, 3).unwrap();

        // also use update to set - to ensure this works
        item.save(storage, &1, 3).unwrap();
        item.update(storage, 3, |_| -> StdResult<u64> { Ok(8) })
            .unwrap();

        item.remove(storage, 4).unwrap();
        item.save(storage, &13, 4).unwrap();

        // checkpoint 5
        item.add_checkpoint(storage, 5).unwrap();
        item.remove(storage, 5).unwrap();
        item.update(storage, 5, |_| -> StdResult<u64> { Ok(22) })
            .unwrap();
        // and delete it later (unknown if all data present)
        item.remove_checkpoint(storage, 5).unwrap();
    }

    const FINAL_VALUE: Option<u64> = Some(22);

    const VALUE_START_3: Option<u64> = Some(7);

    const VALUE_START_5: Option<u64> = Some(13);

    fn assert_final_value(item: &TestItem, storage: &dyn Storage) {
        assert_eq!(FINAL_VALUE, item.may_load(storage).unwrap());
    }

    #[track_caller]
    fn assert_value_at_height(
        item: &TestItem,
        storage: &dyn Storage,
        height: u64,
        value: Option<u64>,
    ) {
        assert_eq!(value, item.may_load_at_height(storage, height).unwrap());
    }

    fn assert_missing_checkpoint(item: &TestItem, storage: &dyn Storage, height: u64) {
        assert!(item.may_load_at_height(storage, height).is_err());
    }

    #[test]
    fn never_works_like_normal_item() {
        let mut storage = MockStorage::new();
        init_data(&NEVER, &mut storage);
        assert_final_value(&NEVER, &storage);

        // historical queries return error
        assert_missing_checkpoint(&NEVER, &storage, 3);
        assert_missing_checkpoint(&NEVER, &storage, 5);
    }

    #[test]
    fn every_blocks_stores_present_and_past() {
        let mut storage = MockStorage::new();
        init_data(&EVERY, &mut storage);
        assert_final_value(&EVERY, &storage);

        // historical queries return historical values
        assert_value_at_height(&EVERY, &storage, 3, VALUE_START_3);
        assert_value_at_height(&EVERY, &storage, 5, VALUE_START_5);
    }

    #[test]
    fn selected_shows_3_not_5() {
        let mut storage = MockStorage::new();
        init_data(&SELECT, &mut storage);
        assert_final_value(&SELECT, &storage);

        // historical queries return historical values
        assert_value_at_height(&SELECT, &storage, 3, VALUE_START_3);
        // never checkpointed
        assert_missing_checkpoint(&NEVER, &storage, 1);
        // deleted checkpoint
        assert_missing_checkpoint(&NEVER, &storage, 5);
    }

    #[test]
    fn handle_multiple_writes_in_one_block() {
        let mut storage = MockStorage::new();

        println!("SETUP");
        EVERY.save(&mut storage, &5, 1).unwrap();
        EVERY.save(&mut storage, &7, 2).unwrap();
        EVERY.save(&mut storage, &2, 2).unwrap();

        // update and save - query at 3 => 2, at 4 => 12
        EVERY
            .update(&mut storage, 3, |_| -> StdResult<u64> { Ok(9) })
            .unwrap();
        EVERY.save(&mut storage, &12, 3).unwrap();
        assert_eq!(Some(5), EVERY.may_load_at_height(&storage, 2).unwrap());
        assert_eq!(Some(2), EVERY.may_load_at_height(&storage, 3).unwrap());
        assert_eq!(Some(12), EVERY.may_load_at_height(&storage, 4).unwrap());

        // save and remove - query at 4 => 1, at 5 => None
        EVERY.save(&mut storage, &17, 4).unwrap();
        EVERY.remove(&mut storage, 4).unwrap();
        assert_eq!(Some(12), EVERY.may_load_at_height(&storage, 4).unwrap());
        assert_eq!(None, EVERY.may_load_at_height(&storage, 5).unwrap());

        // remove and update - query at 5 => 2, at 6 => 13
        EVERY.remove(&mut storage, 5).unwrap();
        EVERY
            .update(&mut storage, 5, |_| -> StdResult<u64> { Ok(2) })
            .unwrap();
        assert_eq!(None, EVERY.may_load_at_height(&storage, 5).unwrap());
        assert_eq!(Some(2), EVERY.may_load_at_height(&storage, 6).unwrap());
    }

    #[test]
    #[cfg(feature = "iterator")]
    fn changelog_range_works() {
        use cosmwasm_std::Order;

        let mut store = MockStorage::new();

        // simple data for testing
        EVERY.save(&mut store, &5, 1u64).unwrap();
        EVERY.save(&mut store, &7, 2u64).unwrap();
        EVERY
            .update(&mut store, 3u64, |_| -> StdResult<u64> { Ok(8) })
            .unwrap();
        EVERY.remove(&mut store, 4u64).unwrap();

        // let's try to iterate over the changelog
        let all: StdResult<Vec<_>> = EVERY
            .changelog()
            .range(&store, None, None, Order::Ascending)
            .collect();
        let all = all.unwrap();
        assert_eq!(4, all.len());
        assert_eq!(
            all,
            vec![
                (1, ChangeSet { old: None }),
                (2, ChangeSet { old: Some(5) }),
                (3, ChangeSet { old: Some(7) }),
                (4, ChangeSet { old: Some(8) })
            ]
        );

        // let's try to iterate over a changelog range
        let all: StdResult<Vec<_>> = EVERY
            .changelog()
            .range(&store, Some(Bound::exclusive(3u64)), None, Order::Ascending)
            .collect();
        let all = all.unwrap();
        assert_eq!(1, all.len());
        assert_eq!(all, vec![(4, ChangeSet { old: Some(8) }),]);
    }
}
