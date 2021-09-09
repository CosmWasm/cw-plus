#![cfg(feature = "iterator")]

use serde::de::DeserializeOwned;
use serde::Serialize;

use cosmwasm_std::{Order, StdError, StdResult, Storage};

use crate::keys::U64Key;
use crate::map::Map;
use crate::snapshot::ChangeSet;
use crate::{Bound, Item, Strategy};

/// Item that maintains a snapshot of one or more checkpoints.
/// We can query historical data as well as current state.
/// What data is snapshotted depends on the Strategy.
pub struct SnapshotItem<'a, T> {
    primary: Item<'a, T>,

    // Maps height to number of checkpoints (only used for selected)
    checkpoints: Map<'a, U64Key, u32>,

    // This stores all changes, by height. Must differentiate between no data written,
    // and explicit None (just inserted)
    changelog: Map<'a, U64Key, ChangeSet<T>>,

    // How aggressive we are about checkpointing all data
    strategy: Strategy,
}

impl<'a, T> SnapshotItem<'a, T> {
    /// Usage: SnapshotItem::new(snapshot_names!("foobar"), Strategy::EveryBlock)
    pub const fn new(
        storage_key: &'a str,
        checkpoints: &'a str,
        changelog: &'a str,
        strategy: Strategy,
    ) -> Self {
        SnapshotItem {
            primary: Item::new(storage_key),
            checkpoints: Map::new(checkpoints),
            changelog: Map::new(changelog),
            strategy,
        }
    }

    pub fn add_checkpoint(&self, store: &mut dyn Storage, height: u64) -> StdResult<()> {
        self.checkpoints
            .update::<_, StdError>(store, height.into(), |count| {
                Ok(count.unwrap_or_default() + 1)
            })?;
        Ok(())
    }

    pub fn remove_checkpoint(&self, store: &mut dyn Storage, height: u64) -> StdResult<()> {
        let count = self
            .checkpoints
            .may_load(store, height.into())?
            .unwrap_or_default();
        if count <= 1 {
            self.checkpoints.remove(store, height.into());
            Ok(())
        } else {
            self.checkpoints.save(store, height.into(), &(count - 1))
        }
    }
}

impl<'a, T> SnapshotItem<'a, T>
where
    T: Serialize + DeserializeOwned + Clone,
{
    /// should_checkpoint looks at the strategy and determines if we want to checkpoint
    fn should_checkpoint(&self, store: &dyn Storage) -> StdResult<bool> {
        match self.strategy {
            Strategy::EveryBlock => Ok(true),
            Strategy::Never => Ok(false),
            Strategy::Selected => self.should_checkpoint_selected(store),
        }
    }

    /// this is just pulled out from above for the selected block
    fn should_checkpoint_selected(&self, store: &dyn Storage) -> StdResult<bool> {
        // most recent checkpoint
        let checkpoint = self
            .checkpoints
            .range(store, None, None, Order::Descending)
            .next()
            .transpose()?;
        if let Some((height, _)) = checkpoint {
            // any changelog for the given key since then?
            let start = Bound::inclusive(U64Key::from(height));
            let first = self
                .changelog
                .range(store, Some(start), None, Order::Ascending)
                .next()
                .transpose()?;
            if first.is_none() {
                // there must be at least one open checkpoint and no changelog for the given address since then
                return Ok(true);
            }
        }
        // otherwise, we don't save this
        Ok(false)
    }

    /// load old value and store changelog
    fn write_change(&self, store: &mut dyn Storage, height: u64) -> StdResult<()> {
        // if there is already data in the changelog for this block, do not write more
        if self
            .changelog
            .may_load(store, U64Key::from(height))?
            .is_some()
        {
            return Ok(());
        }
        // otherwise, store the previous value
        let old = self.primary.may_load(store)?;
        self.changelog
            .save(store, U64Key::from(height), &ChangeSet { old })
    }

    pub fn save(&self, store: &mut dyn Storage, data: &T, height: u64) -> StdResult<()> {
        if self.should_checkpoint(store)? {
            self.write_change(store, height)?;
        }
        self.primary.save(store, data)
    }

    pub fn remove(&self, store: &mut dyn Storage, height: u64) -> StdResult<()> {
        if self.should_checkpoint(store)? {
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

    // may_load_at_height reads historical data from given checkpoints.
    // Only returns `Ok` if we have the data to be able to give the correct answer
    // (Strategy::EveryBlock or Strategy::Selected and h is registered as checkpoint)
    //
    // If there is no checkpoint for that height, then we return StdError::NotFound
    pub fn may_load_at_height(&self, store: &dyn Storage, height: u64) -> StdResult<Option<T>> {
        self.assert_checkpointed(store, height)?;

        // this will look for the first snapshot of the given address >= given height
        // If None, there is no snapshot since that time.
        let start = Bound::inclusive(U64Key::new(height));
        let first = self
            .changelog
            .range(store, Some(start), None, Order::Ascending)
            .next();

        if let Some(r) = first {
            // if we found a match, return this last one
            r.map(|(_, v)| v.old)
        } else {
            // otherwise, return current value
            self.may_load(store)
        }
    }

    // If there is no checkpoint for that height, then we return StdError::NotFound
    pub fn assert_checkpointed(&self, store: &dyn Storage, height: u64) -> StdResult<()> {
        let has = match self.strategy {
            Strategy::EveryBlock => true,
            Strategy::Never => false,
            Strategy::Selected => self.checkpoints.may_load(store, height.into())?.is_some(),
        };
        match has {
            true => Ok(()),
            false => Err(StdError::not_found("checkpoint")),
        }
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
}
