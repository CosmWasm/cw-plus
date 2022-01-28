#![cfg(feature = "iterator")]
mod item;
mod map;

pub use item::SnapshotItem;
pub use map::SnapshotMap;

use crate::bound::Bound;
use crate::de::KeyDeserialize;
use crate::{Map, Prefixer, PrimaryKey};
use cosmwasm_std::{Order, StdError, StdResult, Storage};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

/// Structure holding a map of checkpoints composited from
/// height (as u64) and counter of how many times it has
/// been checkpointed (as u32).
/// Stores all changes in changelog.
#[derive(Debug, Clone)]
pub(crate) struct Snapshot<'a, K, T> {
    checkpoints: Map<'a, u64, u32>,

    // this stores all changes (key, height). Must differentiate between no data written,
    // and explicit None (just inserted)
    pub changelog: Map<'a, (K, u64), ChangeSet<T>>,

    // How aggressive we are about checkpointing all data
    strategy: Strategy,
}

impl<'a, K, T> Snapshot<'a, K, T> {
    pub const fn new(
        checkpoints: &'a str,
        changelog: &'a str,
        strategy: Strategy,
    ) -> Snapshot<'a, K, T> {
        Snapshot {
            checkpoints: Map::new(checkpoints),
            changelog: Map::new(changelog),
            strategy,
        }
    }

    pub fn add_checkpoint(&self, store: &mut dyn Storage, height: u64) -> StdResult<()> {
        self.checkpoints
            .update::<_, StdError>(store, height, |count| Ok(count.unwrap_or_default() + 1))?;
        Ok(())
    }

    pub fn remove_checkpoint(&self, store: &mut dyn Storage, height: u64) -> StdResult<()> {
        let count = self
            .checkpoints
            .may_load(store, height)?
            .unwrap_or_default();
        if count <= 1 {
            self.checkpoints.remove(store, height);
            Ok(())
        } else {
            self.checkpoints.save(store, height, &(count - 1))
        }
    }
}

impl<'a, K, T> Snapshot<'a, K, T>
where
    T: Serialize + DeserializeOwned + Clone,
    K: PrimaryKey<'a> + Prefixer<'a> + KeyDeserialize,
{
    /// should_checkpoint looks at the strategy and determines if we want to checkpoint
    pub fn should_checkpoint(&self, store: &dyn Storage, k: &K) -> StdResult<bool> {
        match self.strategy {
            Strategy::EveryBlock => Ok(true),
            Strategy::Never => Ok(false),
            Strategy::Selected => self.should_checkpoint_selected(store, k),
        }
    }

    /// this is just pulled out from above for the selected block
    fn should_checkpoint_selected(&self, store: &dyn Storage, k: &K) -> StdResult<bool> {
        // most recent checkpoint
        let checkpoint = self
            .checkpoints
            .range(store, None, None, Order::Descending)
            .next()
            .transpose()?;
        if let Some((height, _)) = checkpoint {
            // any changelog for the given key since then?
            let start = Bound::inclusive(height);
            let first = self
                .changelog
                .prefix(k.clone())
                .range_raw(store, Some(start), None, Order::Ascending)
                .next()
                .transpose()?;
            if first.is_none() {
                // there must be at least one open checkpoint and no changelog for the given height since then
                return Ok(true);
            }
        }
        // otherwise, we don't save this
        Ok(false)
    }

    // If there is no checkpoint for that height, then we return StdError::NotFound
    pub fn assert_checkpointed(&self, store: &dyn Storage, height: u64) -> StdResult<()> {
        let has = match self.strategy {
            Strategy::EveryBlock => true,
            Strategy::Never => false,
            Strategy::Selected => self.checkpoints.may_load(store, height)?.is_some(),
        };
        match has {
            true => Ok(()),
            false => Err(StdError::not_found("checkpoint")),
        }
    }

    pub fn has_changelog(&self, store: &mut dyn Storage, key: K, height: u64) -> StdResult<bool> {
        Ok(self.changelog.may_load(store, (key, height))?.is_some())
    }

    pub fn write_changelog(
        &self,
        store: &mut dyn Storage,
        key: K,
        height: u64,
        old: Option<T>,
    ) -> StdResult<()> {
        self.changelog
            .save(store, (key, height), &ChangeSet { old })
    }

    // may_load_at_height reads historical data from given checkpoints.
    // Returns StdError::NotFound if we have no checkpoint, and can give no data.
    // Returns Ok(None) if there is a checkpoint, but no cached data (no changes since the
    // checkpoint. Caller should query current state).
    // Return Ok(Some(x)) if there is a checkpoint and data written to changelog, returning the state at that time
    pub fn may_load_at_height(
        &self,
        store: &dyn Storage,
        key: K,
        height: u64,
    ) -> StdResult<Option<Option<T>>> {
        self.assert_checkpointed(store, height)?;

        // this will look for the first snapshot of height >= given height
        // If None, there is no snapshot since that time.
        let start = Bound::inclusive(height);
        let first = self
            .changelog
            .prefix(key)
            .range_raw(store, Some(start), None, Order::Ascending)
            .next();

        if let Some(r) = first {
            // if we found a match, return this last one
            r.map(|(_, v)| Some(v.old))
        } else {
            Ok(None)
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub enum Strategy {
    EveryBlock,
    Never,
    /// Only writes for linked blocks - does a few more reads to save some writes.
    /// Probably uses more gas, but less total disk usage.
    ///
    /// Note that you need a trusted source (eg. own contract) to set/remove checkpoints.
    /// Useful when the checkpoint setting happens in the same contract as the snapshotting.
    Selected,
}

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct ChangeSet<T> {
    pub old: Option<T>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::MockStorage;

    type TestSnapshot = Snapshot<'static, &'static str, u64>;

    const NEVER: TestSnapshot = Snapshot::new("never__check", "never__change", Strategy::Never);
    const EVERY: TestSnapshot =
        Snapshot::new("every__check", "every__change", Strategy::EveryBlock);
    const SELECT: TestSnapshot =
        Snapshot::new("select__check", "select__change", Strategy::Selected);

    const DUMMY_KEY: &str = "dummy";

    #[test]
    fn should_checkpoint() {
        let storage = MockStorage::new();

        assert_eq!(NEVER.should_checkpoint(&storage, &DUMMY_KEY), Ok(false));
        assert_eq!(EVERY.should_checkpoint(&storage, &DUMMY_KEY), Ok(true));
        assert_eq!(SELECT.should_checkpoint(&storage, &DUMMY_KEY), Ok(false));
    }

    #[test]
    fn assert_checkpointed() {
        let mut storage = MockStorage::new();

        assert_eq!(
            NEVER.assert_checkpointed(&storage, 1),
            Err(StdError::not_found("checkpoint"))
        );
        assert_eq!(EVERY.assert_checkpointed(&storage, 1), Ok(()));
        assert_eq!(
            SELECT.assert_checkpointed(&storage, 1),
            Err(StdError::not_found("checkpoint"))
        );

        // Add a checkpoint at 1
        NEVER.add_checkpoint(&mut storage, 1).unwrap();
        EVERY.add_checkpoint(&mut storage, 1).unwrap();
        SELECT.add_checkpoint(&mut storage, 1).unwrap();

        assert_eq!(
            NEVER.assert_checkpointed(&storage, 1),
            Err(StdError::not_found("checkpoint"))
        );
        assert_eq!(EVERY.assert_checkpointed(&storage, 1), Ok(()));
        assert_eq!(SELECT.assert_checkpointed(&storage, 1), Ok(()));

        // Remove checkpoint
        NEVER.remove_checkpoint(&mut storage, 1).unwrap();
        EVERY.remove_checkpoint(&mut storage, 1).unwrap();
        SELECT.remove_checkpoint(&mut storage, 1).unwrap();

        assert_eq!(
            NEVER.assert_checkpointed(&storage, 1),
            Err(StdError::not_found("checkpoint"))
        );
        assert_eq!(EVERY.assert_checkpointed(&storage, 1), Ok(()));
        assert_eq!(
            SELECT.assert_checkpointed(&storage, 1),
            Err(StdError::not_found("checkpoint"))
        );
    }

    #[test]
    fn has_changelog() {
        let mut storage = MockStorage::new();

        assert_eq!(NEVER.has_changelog(&mut storage, DUMMY_KEY, 1), Ok(false));
        assert_eq!(EVERY.has_changelog(&mut storage, DUMMY_KEY, 1), Ok(false));
        assert_eq!(SELECT.has_changelog(&mut storage, DUMMY_KEY, 1), Ok(false));

        assert_eq!(NEVER.has_changelog(&mut storage, DUMMY_KEY, 2), Ok(false));
        assert_eq!(EVERY.has_changelog(&mut storage, DUMMY_KEY, 2), Ok(false));
        assert_eq!(SELECT.has_changelog(&mut storage, DUMMY_KEY, 2), Ok(false));

        assert_eq!(NEVER.has_changelog(&mut storage, DUMMY_KEY, 3), Ok(false));
        assert_eq!(EVERY.has_changelog(&mut storage, DUMMY_KEY, 3), Ok(false));
        assert_eq!(SELECT.has_changelog(&mut storage, DUMMY_KEY, 3), Ok(false));

        // Write a changelog at 2
        NEVER
            .write_changelog(&mut storage, DUMMY_KEY, 2, Some(3))
            .unwrap();
        EVERY
            .write_changelog(&mut storage, DUMMY_KEY, 2, Some(4))
            .unwrap();
        SELECT
            .write_changelog(&mut storage, DUMMY_KEY, 2, Some(5))
            .unwrap();

        assert_eq!(NEVER.has_changelog(&mut storage, DUMMY_KEY, 1), Ok(false));
        assert_eq!(EVERY.has_changelog(&mut storage, DUMMY_KEY, 1), Ok(false));
        assert_eq!(SELECT.has_changelog(&mut storage, DUMMY_KEY, 1), Ok(false));

        assert_eq!(NEVER.has_changelog(&mut storage, DUMMY_KEY, 2), Ok(true));
        assert_eq!(EVERY.has_changelog(&mut storage, DUMMY_KEY, 2), Ok(true));
        assert_eq!(SELECT.has_changelog(&mut storage, DUMMY_KEY, 2), Ok(true));

        assert_eq!(NEVER.has_changelog(&mut storage, DUMMY_KEY, 3), Ok(false));
        assert_eq!(EVERY.has_changelog(&mut storage, DUMMY_KEY, 3), Ok(false));
        assert_eq!(SELECT.has_changelog(&mut storage, DUMMY_KEY, 3), Ok(false));
    }

    #[test]
    fn may_load_at_height() {
        let mut storage = MockStorage::new();

        assert_eq!(
            NEVER.may_load_at_height(&storage, DUMMY_KEY, 3),
            Err(StdError::not_found("checkpoint"))
        );
        assert_eq!(EVERY.may_load_at_height(&storage, DUMMY_KEY, 3), Ok(None));
        assert_eq!(
            SELECT.may_load_at_height(&storage, DUMMY_KEY, 3),
            Err(StdError::not_found("checkpoint"))
        );

        // Add a checkpoint at 3
        NEVER.add_checkpoint(&mut storage, 3).unwrap();
        EVERY.add_checkpoint(&mut storage, 3).unwrap();
        SELECT.add_checkpoint(&mut storage, 3).unwrap();

        assert_eq!(
            NEVER.may_load_at_height(&storage, DUMMY_KEY, 3),
            Err(StdError::not_found("checkpoint"))
        );
        assert_eq!(EVERY.may_load_at_height(&storage, DUMMY_KEY, 3), Ok(None));
        assert_eq!(SELECT.may_load_at_height(&storage, DUMMY_KEY, 3), Ok(None));

        // Write a changelog at 3
        NEVER
            .write_changelog(&mut storage, DUMMY_KEY, 3, Some(100))
            .unwrap();
        EVERY
            .write_changelog(&mut storage, DUMMY_KEY, 3, Some(101))
            .unwrap();
        SELECT
            .write_changelog(&mut storage, DUMMY_KEY, 3, Some(102))
            .unwrap();

        assert_eq!(
            NEVER.may_load_at_height(&storage, DUMMY_KEY, 3),
            Err(StdError::not_found("checkpoint"))
        );
        assert_eq!(
            EVERY.may_load_at_height(&storage, DUMMY_KEY, 3),
            Ok(Some(Some(101)))
        );
        assert_eq!(
            SELECT.may_load_at_height(&storage, DUMMY_KEY, 3),
            Ok(Some(Some(102)))
        );
        // Check that may_load_at_height at a previous value will return the first change after that.
        // (Only with EVERY).
        assert_eq!(
            NEVER.may_load_at_height(&storage, DUMMY_KEY, 2),
            Err(StdError::not_found("checkpoint"))
        );
        assert_eq!(
            EVERY.may_load_at_height(&storage, DUMMY_KEY, 2),
            Ok(Some(Some(101)))
        );
        assert_eq!(
            SELECT.may_load_at_height(&storage, DUMMY_KEY, 2),
            Err(StdError::not_found("checkpoint"))
        );

        // Write a changelog at 4, removing the value
        NEVER
            .write_changelog(&mut storage, DUMMY_KEY, 4, None)
            .unwrap();
        EVERY
            .write_changelog(&mut storage, DUMMY_KEY, 4, None)
            .unwrap();
        SELECT
            .write_changelog(&mut storage, DUMMY_KEY, 4, None)
            .unwrap();
        // And add a checkpoint at 4
        NEVER.add_checkpoint(&mut storage, 4).unwrap();
        EVERY.add_checkpoint(&mut storage, 4).unwrap();
        SELECT.add_checkpoint(&mut storage, 4).unwrap();

        assert_eq!(
            NEVER.may_load_at_height(&storage, DUMMY_KEY, 4),
            Err(StdError::not_found("checkpoint"))
        );
        assert_eq!(
            EVERY.may_load_at_height(&storage, DUMMY_KEY, 4),
            Ok(Some(None))
        );
        assert_eq!(
            SELECT.may_load_at_height(&storage, DUMMY_KEY, 4),
            Ok(Some(None))
        );

        // Confirm old value at 3
        assert_eq!(
            NEVER.may_load_at_height(&storage, DUMMY_KEY, 3),
            Err(StdError::not_found("checkpoint"))
        );
        assert_eq!(
            EVERY.may_load_at_height(&storage, DUMMY_KEY, 3),
            Ok(Some(Some(101)))
        );
        assert_eq!(
            SELECT.may_load_at_height(&storage, DUMMY_KEY, 3),
            Ok(Some(Some(102)))
        );
    }
}
