use crate::{Bound, Map, Prefixer, PrimaryKey, U64Key};
use cosmwasm_std::{Order, StdError, StdResult, Storage};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub(crate) struct Snapshot<'a, K, T> {
    checkpoints: Map<'a, U64Key, u32>,

    // this stores all changes (key, height). Must differentiate between no data written,
    // and explicit None (just inserted)
    pub changelog: Map<'a, (K, U64Key), ChangeSet<T>>,

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

impl<'a, K, T> Snapshot<'a, K, T>
where
    T: Serialize + DeserializeOwned + Clone,
    K: PrimaryKey<'a> + Prefixer<'a>,
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
            let start = Bound::inclusive(U64Key::from(height));
            let first = self
                .changelog
                .prefix(k.clone())
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

    pub fn has_changelog(&self, store: &mut dyn Storage, key: K, height: u64) -> StdResult<bool> {
        Ok(self
            .changelog
            .may_load(store, (key, U64Key::from(height)))?
            .is_some())
    }

    pub fn write_changelog(
        &self,
        store: &mut dyn Storage,
        key: K,
        height: u64,
        old: Option<T>,
    ) -> StdResult<()> {
        self.changelog
            .save(store, (key, U64Key::from(height)), &ChangeSet { old })
    }

    // may_load_at_height reads historical data from given checkpoints.
    // Only returns `Ok` if we have the data to be able to give the correct answer
    // (Strategy::EveryBlock or Strategy::Selected and h is registered as checkpoint)
    //
    // If there is no checkpoint for that height, then we return StdError::NotFound
    pub fn may_load_at_height(
        &self,
        store: &dyn Storage,
        key: K,
        height: u64,
    ) -> StdResult<Option<Option<T>>> {
        self.assert_checkpointed(store, height)?;

        // this will look for the first snapshot of the given address >= given height
        // If None, there is no snapshot since that time.
        let start = Bound::inclusive(U64Key::new(height));
        let first = self
            .changelog
            .prefix(key)
            .range(store, Some(start), None, Order::Ascending)
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
pub(crate) struct ChangeSet<T> {
    pub old: Option<T>,
}
