mod bound;
mod de;
mod deque;
mod endian;
mod helpers;
mod indexed_map;
mod indexed_snapshot;
mod indexes;
mod int_key;
mod item;
mod iter_helpers;
mod keys;
mod map;
mod path;
mod prefix;
mod snapshot;

#[cfg(feature = "iterator")]
pub use bound::{Bound, Bounder, PrefixBound, RawBound};
pub use de::KeyDeserialize;
pub use deque::VecDeque;
#[cfg(feature = "iterator")]
pub use deque::VecDequeIter;
pub use endian::Endian;
#[cfg(feature = "iterator")]
pub use indexed_map::{IndexList, IndexedMap};
#[cfg(feature = "iterator")]
pub use indexed_snapshot::IndexedSnapshotMap;
#[cfg(feature = "iterator")]
pub use indexes::Index;
#[cfg(feature = "iterator")]
pub use indexes::MultiIndex;
#[cfg(feature = "iterator")]
pub use indexes::UniqueIndex;
pub use int_key::IntKey;
pub use item::Item;
pub use keys::{Key, Prefixer, PrimaryKey};
pub use map::Map;
pub use path::Path;
#[cfg(feature = "iterator")]
pub use prefix::{range_with_prefix, Prefix};
#[cfg(feature = "iterator")]
pub use snapshot::{SnapshotItem, SnapshotMap, Strategy};

// cw_storage_macro reexports
#[cfg(all(feature = "iterator", feature = "macro"))]
#[macro_use]
extern crate cw_storage_macro;
#[cfg(all(feature = "iterator", feature = "macro"))]
/// Auto generate an `IndexList` impl for your indexes struct.
///
/// # Example
///
/// ```rust
/// use cosmwasm_std::Addr;
/// use cw_storage_plus::{MultiIndex, UniqueIndex, index_list};
/// use serde::{Serialize, Deserialize};
///
/// #[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
/// struct TestStruct {
///     id: u64,
///     id2: u32,
///     addr: Addr,
/// }
///
/// #[index_list(TestStruct)] // <- Add this line right here.
/// struct TestIndexes<'a> {
///     id: MultiIndex<'a, u32, TestStruct, u64>,
///     addr: UniqueIndex<'a, Addr, TestStruct>,
/// }
/// ```
///
pub use cw_storage_macro::index_list;
#[cfg(all(feature = "iterator", feature = "macro"))]
pub use cw_storage_macro::*;
