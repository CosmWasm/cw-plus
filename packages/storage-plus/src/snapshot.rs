use serde::{Deserialize, Serialize};

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
