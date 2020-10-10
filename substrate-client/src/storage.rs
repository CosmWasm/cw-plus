use crate::common::traits::aux_store::AuxStore;
use crate::common::traits::header_backend::HeaderBackend;
use crate::common::traits::header_metadata::HeaderMetadata;
use crate::common::traits::storage::Storage as StorageT;
use crate::common::types::block_status::BlockStatus;
use crate::common::types::blockchain_error::BlockchainError;
use crate::common::types::blockchain_info::BlockchainInfo;
use crate::common::types::blockchain_result::BlockchainResult;
use crate::common::types::cached_header_metadata::CachedHeaderMetadata;
use crate::common::types::new_block_state::NewBlockState;
use crate::db::Data;
use kvdb::{DBTransaction, KeyValueDB};
use parity_scale_codec::{Decode, Encode};
use sp_runtime::generic::BlockId;
use sp_runtime::traits::{Block as BlockT, Header as HeaderT, NumberFor, One, Zero};
use std::io;

const META_COLUMN: u32 = 0;
const HEADER_COLUMN: u32 = 1;
const AUX_COLUMN: u32 = 2;
const LOOKUP_COLUMN: u32 = 3;

const META_KEY: &[u8] = b"meta";

/// Database metadata.
#[derive(Debug, Encode, Decode)]
struct StorageMeta<N, H>
where
    N: Encode + Decode,
    H: Encode + Decode,
{
    /// Hash of the best known block.
    pub best_hash: H,
    /// Number of the best known block.
    pub best_number: N,
    /// Hash of the best finalized block.
    pub finalized_hash: H,
    /// Number of the best finalized block.
    pub finalized_number: N,
    /// Hash of the genesis block.
    pub genesis_hash: H,
    /// headers stored at the moment
    pub total_stored: u64,
    /// Oldest stored header's corresponding block hash
    pub oldest_stored_hash: H,
}

fn db_err(err: io::Error) -> BlockchainError {
    BlockchainError::Backend(format!("{}", err))
}

fn codec_error(err: parity_scale_codec::Error) -> BlockchainError {
    BlockchainError::DataDecode(err.to_string())
}

pub struct Storage {
    data: Data,
    max_headers_allowed_to_store: u64,
}

impl Storage {
    pub fn new(data: Data, max_headers_allowed_to_store: u64) -> Result<Self, BlockchainError> {
        if max_headers_allowed_to_store < 2 {
            Err(BlockchainError::Backend(
                "Maximum amount of blocks allowed to store need to be at least 2".into(),
            ))
        } else {
            Ok(Self {
                data,
                max_headers_allowed_to_store,
            })
        }
    }

    fn fetch_meta<N, H>(&self) -> BlockchainResult<Option<StorageMeta<N, H>>>
    where
        N: Encode + Decode,
        H: Encode + Decode,
    {
        let possible_encoded_meta = self.data.db.get(META_COLUMN, META_KEY).map_err(db_err)?;
        if possible_encoded_meta.is_none() {
            Ok(None)
        } else {
            let encoded_meta = possible_encoded_meta.unwrap();
            Ok(Some(
                StorageMeta::decode(&mut encoded_meta.as_slice()).map_err(codec_error)?,
            ))
        }
    }

    fn store_meta<N, H>(&self, meta: StorageMeta<N, H>) -> BlockchainResult<()>
    where
        N: Encode + Decode,
        H: Encode + Decode,
    {
        let mut tx = self.data.db.transaction();
        Self::tx_store_meta(&mut tx, &meta);
        self.data.db.write(tx).map_err(db_err)
    }

    fn tx_store_meta<N, H>(tx: &mut DBTransaction, meta: &StorageMeta<N, H>)
    where
        N: Encode + Decode,
        H: Encode + Decode,
    {
        tx.put(META_COLUMN, META_KEY, meta.encode().as_slice());
    }

    fn tx_store_header<Block>(tx: &mut DBTransaction, header: &Block::Header)
    where
        Block: BlockT,
    {
        let id = Self::header_hash_to_id::<Block>(&header.hash());
        tx.put(HEADER_COLUMN, id.as_slice(), header.encode().as_slice());
    }

    fn tx_delete_header<Block>(tx: &mut DBTransaction, hash: &Block::Hash)
    where
        Block: BlockT,
    {
        let id = Self::header_hash_to_id::<Block>(hash);
        tx.delete(HEADER_COLUMN, id.as_slice());
    }

    fn header_hash_to_id<Block>(hash: &Block::Hash) -> Vec<u8>
    where
        Block: BlockT,
    {
        hash.encode()
    }

    fn id<Block>(&self, block_id: BlockId<Block>) -> BlockchainResult<Option<Vec<u8>>>
    where
        Block: BlockT,
    {
        match block_id {
            BlockId::Hash(h) => Ok(Some(Self::header_hash_to_id::<Block>(&h))),
            BlockId::Number(n) => {
                let data = self
                    .data
                    .db
                    .get(LOOKUP_COLUMN, n.encode().as_slice())
                    .map_err(db_err)?;
                if data.is_none() {
                    Ok(None)
                } else {
                    Ok(Some(data.unwrap().to_vec()))
                }
            }
        }
    }

    fn header_hash<Block>(&self, number: NumberFor<Block>) -> BlockchainResult<Option<Block::Hash>>
    where
        Block: BlockT,
    {
        let data = self
            .data
            .db
            .get(LOOKUP_COLUMN, number.encode().as_slice())
            .map_err(db_err)?;
        if data.is_none() {
            Ok(None)
        } else {
            let encoded_header = data.unwrap();
            Ok(Some(
                Block::Hash::decode(&mut encoded_header.as_slice()).map_err(codec_error)?,
            ))
        }
    }
}

impl AuxStore for Storage {
    fn insert_aux<
        'a,
        'b: 'a,
        'c: 'a,
        I: IntoIterator<Item = &'a (&'c [u8], &'c [u8])>,
        D: IntoIterator<Item = &'a &'b [u8]>,
    >(
        &self,
        insert: I,
        delete: D,
    ) -> BlockchainResult<()> {
        let mut tx = self.data.db.transaction();
        for (k, v) in insert {
            tx.put(AUX_COLUMN, *k, *v);
        }

        for k in delete {
            tx.delete(AUX_COLUMN, *k)
        }

        self.data.db.write(tx).map_err(db_err)
    }

    fn get_aux(&self, key: &[u8]) -> BlockchainResult<Option<Vec<u8>>> {
        self.data.db.get(AUX_COLUMN, key).map_err(db_err)
    }
}

impl<Block> HeaderBackend<Block> for Storage
where
    Block: BlockT,
{
    fn header(&self, id: BlockId<Block>) -> BlockchainResult<Option<Block::Header>> {
        let possible_header_key = self.id(id)?;
        if possible_header_key.is_none() {
            Ok(None)
        } else {
            let header_key = possible_header_key.unwrap();
            let possible_encoded_header = self
                .data
                .db
                .get(HEADER_COLUMN, header_key.as_slice())
                .map_err(db_err)?;
            if possible_encoded_header.is_none() {
                Ok(None)
            } else {
                let encoded_header = possible_encoded_header.unwrap();
                let header =
                    Block::Header::decode(&mut encoded_header.as_slice()).map_err(codec_error)?;
                Ok(Some(header))
            }
        }
    }

    fn info(&self) -> BlockchainInfo<Block> {
        let meta = self.fetch_meta();
        let default_info = BlockchainInfo {
            best_hash: Default::default(),
            best_number: Zero::zero(),
            genesis_hash: Default::default(),
            finalized_hash: Default::default(),
            finalized_number: Zero::zero(),
            number_leaves: 0,
        };
        if meta.is_ok() {
            let meta = meta.unwrap();
            if meta.is_none() {
                default_info
            } else {
                let meta = meta.unwrap();
                BlockchainInfo {
                    best_hash: meta.best_hash,
                    best_number: meta.best_number,
                    genesis_hash: meta.genesis_hash,
                    finalized_hash: meta.finalized_hash,
                    finalized_number: meta.finalized_number,
                    number_leaves: 0,
                }
            }
        } else {
            default_info
        }
    }

    fn status(&self, id: BlockId<Block>) -> BlockchainResult<BlockStatus> {
        let possible_header = self.header(id)?;
        if possible_header.is_none() {
            Ok(BlockStatus::Unknown)
        } else {
            Ok(BlockStatus::InChain)
        }
    }

    fn number(
        &self,
        hash: Block::Hash,
    ) -> BlockchainResult<Option<<Block::Header as HeaderT>::Number>> {
        let possible_header: Option<Block::Header> = self.header(BlockId::<Block>::Hash(hash))?;
        if possible_header.is_none() {
            Ok(None)
        } else {
            let header = possible_header.unwrap();
            Ok(Some(*header.number()))
        }
    }

    fn hash(&self, number: NumberFor<Block>) -> BlockchainResult<Option<Block::Hash>> {
        self.header_hash::<Block>(number)
    }
}

impl<Block> StorageT<Block> for Storage
where
    Block: BlockT,
{
    /// Store new header. Should refuse to revert any finalized blocks.
    ///
    /// Takes new authorities, the leaf state of the new block, and
    /// any auxiliary storage updates to place in the same operation.
    fn import_header(&self, header: Block::Header, state: NewBlockState) -> BlockchainResult<()> {
        assert!(
            state.is_best(),
            "Since, we are only following one fork block state must need to be best"
        );

        let possible_meta = self.fetch_meta()?;
        let mut meta: StorageMeta<NumberFor<Block>, Block::Hash> = if possible_meta.is_none() {
            StorageMeta {
                best_hash: Default::default(),
                best_number: Zero::zero(),
                finalized_hash: Default::default(),
                finalized_number: Zero::zero(),
                genesis_hash: Default::default(),
                total_stored: 0,
                oldest_stored_hash: Default::default(),
            }
        } else {
            possible_meta.unwrap()
        };

        let mut tx = self.data.db.transaction();

        // We need to go down this in-efficient route, because
        // to have a double linked list of headers require more storage and
        // memory which we don't have.
        // So, we need to backtrack every time we are above the limit.
        if meta.total_stored >= self.max_headers_allowed_to_store {
            let mut current_hash = meta.best_hash;
            let amount_of_headers_to_backtrack = self.max_headers_allowed_to_store - 1;
            let amount_of_headers_to_delete =
                (meta.total_stored - self.max_headers_allowed_to_store) + 1;
            // First backtrack to the newest header we need to delete
            for _ in 0..amount_of_headers_to_backtrack {
                let possible_header = self.header(BlockId::<Block>::Hash(current_hash))?;
                if possible_header.is_none() {
                    return Err(BlockchainError::Backend(format!(
                        "FATAL: Storage inconsistency. Unable to retrieve stored block"
                    )));
                }
                let header = possible_header.unwrap();
                meta.oldest_stored_hash = current_hash;
                current_hash = *header.parent_hash();
            }
            // Now, Let's delete newest header and all headers older than newest header
            for _ in 0..amount_of_headers_to_delete {
                let possible_header = self.header(BlockId::<Block>::Hash(current_hash))?;
                if possible_header.is_none() {
                    return Err(BlockchainError::Backend(format!(
                        "FATAL: Storage inconsistency. Unable to retrieve stored block"
                    )));
                }
                Self::tx_delete_header::<Block>(&mut tx, &current_hash);
                meta.total_stored -= 1;
                let header = possible_header.unwrap();
                current_hash = *header.parent_hash();
            }
        }

        let possible_header = self.header(BlockId::<Block>::Hash(header.hash()))?;
        if possible_header.is_some() {
            // We have already imported this block
            return Ok(());
        }

        let first_imported_header = meta.best_hash == Default::default();

        // We need to check if this is child of last best header
        if !first_imported_header {
            let possible_parent_header = self.header(BlockId::<Block>::Hash(meta.best_hash))?;
            if possible_parent_header.is_none() {
                return Err(BlockchainError::UnknownBlock(format!(
                    "Could not find parent of importing block"
                )));
            }
            let parent_header = possible_parent_header.unwrap();
            if *header.parent_hash() != parent_header.hash()
                || header.number() <= parent_header.number()
            {
                return Err(BlockchainError::NotInFinalizedChain);
            }
            if *header.number() != meta.best_number + One::one() {
                return Err(BlockchainError::NonSequentialImport(format!(
                    "to be imported block need to be child of last best block or first block itself. Expected block number: {}. Got: {}",
                    meta.best_number + One::one(),
                    *header.number()
                )));
            }
        } else {
            meta.genesis_hash = header.hash();
            meta.oldest_stored_hash = header.hash();
        }

        meta.total_stored += 1;
        meta.best_hash = header.hash();
        meta.best_number = *header.number();

        Self::tx_store_meta(&mut tx, &meta);
        Self::tx_store_header::<Block>(&mut tx, &header);
        self.data.db.write(tx).map_err(db_err)
    }

    /// Set an existing block as new best block.
    fn set_head(&self, _block: BlockId<Block>) -> BlockchainResult<()> {
        unimplemented!()
    }

    /// Mark historic header as finalized.
    fn finalize_header(&self, block: BlockId<Block>) -> BlockchainResult<()> {
        let possible_to_be_finalized_header = self.header(block)?;
        if possible_to_be_finalized_header.is_none() {
            return Err(BlockchainError::UnknownBlock(format!(
                "Error: {}",
                "Could not find block header to finalize"
            )));
        }
        let to_be_finalized_header = possible_to_be_finalized_header.unwrap();
        let possible_meta = self.fetch_meta()?;
        if possible_meta.is_none() {
            return Err(BlockchainError::Backend(format!(
                "Error: {}",
                "Unable to get metadata about blockchain"
            )));
        }
        let mut meta: StorageMeta<NumberFor<Block>, Block::Hash> = possible_meta.unwrap();
        let first_block_to_be_finalized = meta.finalized_hash == Default::default();

        if (!first_block_to_be_finalized
            && *to_be_finalized_header.parent_hash() != meta.finalized_hash)
            || (first_block_to_be_finalized && to_be_finalized_header.hash() != meta.genesis_hash)
        {
            return Err(BlockchainError::NonSequentialFinalization(format!("Error: {}", "to be finalized block need to be child of last finalized block or first block itself")));
        }

        meta.finalized_hash = to_be_finalized_header.hash();
        meta.finalized_number = *to_be_finalized_header.number();

        self.store_meta(meta)
    }

    /// Get last finalized header.
    fn last_finalized(&self) -> BlockchainResult<Block::Hash> {
        let possible_meta: Option<StorageMeta<NumberFor<Block>, Block::Hash>> =
            self.fetch_meta()?;
        if possible_meta.is_none() {
            return Err(BlockchainError::Backend(format!(
                "Error: {}",
                "Unable to get metadata about blockchain"
            )));
        }
        Ok(possible_meta.unwrap().finalized_hash)
    }
}

impl<Block> HeaderMetadata<Block> for Storage
where
    Block: BlockT,
{
    type Error = BlockchainError;

    fn header_metadata(
        &self,
        hash: Block::Hash,
    ) -> Result<CachedHeaderMetadata<Block>, Self::Error> {
        let possible_header = self.header(BlockId::<Block>::Hash(hash))?;
        if possible_header.is_none() {
            Err(BlockchainError::UnknownBlock(format!(
                "header not found in db: {}",
                hash
            )))
        } else {
            let header = possible_header.unwrap();
            Ok(CachedHeaderMetadata::from(&header))
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::common::traits::header_backend::HeaderBackend;
    use crate::common::traits::storage::Storage as StorageT;
    use crate::common::types::new_block_state::NewBlockState;
    use crate::db::{create, Data};
    use crate::genesis::GenesisData;
    use crate::storage::Storage;
    use crate::types::{Block, Header};
    use parity_scale_codec::Encode;
    use sp_api::BlockId;
    use sp_runtime::traits::{Block as BlockT, Header as HeaderT, NumberFor, One};

    fn create_next_header(header: Header) -> Header {
        let mut next_header = header.clone();
        next_header.number += 1;
        next_header.parent_hash = header.hash();
        next_header
    }

    #[test]
    fn test_storage_init() {
        let data = Data {
            db: create(11),
            genesis_data: GenesisData {},
        };

        let result = Storage::new(data.clone(), 2);
        assert!(result.is_ok());

        let result = Storage::new(data.clone(), 1);
        assert!(result.is_err());
        assert_eq!(
            result.err().unwrap().to_string(),
            "Backend error: Maximum amount of blocks allowed to store need to be at least 2"
        );

        let result = Storage::new(data.clone(), 0);
        assert!(result.is_err());
        assert_eq!(
            result.err().unwrap().to_string(),
            "Backend error: Maximum amount of blocks allowed to store need to be at least 2"
        );
    }

    #[test]
    fn test_storage_space_management() {
        let data = Data {
            db: create(11),
            genesis_data: GenesisData {},
        };

        let mut produced_headers = vec![];
        let max_headers_allowed_to_store = 7;

        let result = Storage::new(data.clone(), max_headers_allowed_to_store);
        assert!(result.is_ok());
        let storage = result.unwrap();

        let mut current_header = Header::new(
            One::one(),
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
        );

        // Initially meta structure should not exists
        let result = storage.fetch_meta::<NumberFor<Block>, <Block as BlockT>::Hash>();
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());

        // Let's store first max_headers_allowed_to_store number of headers
        for i in 0..max_headers_allowed_to_store {
            current_header = create_next_header(current_header.clone());
            produced_headers.push(current_header.clone());
            assert!(StorageT::<Block>::import_header(
                &storage,
                current_header.clone(),
                NewBlockState::Best
            )
            .is_ok());

            // Check if meta is updated correctly.
            let result = storage.fetch_meta::<NumberFor<Block>, <Block as BlockT>::Hash>();
            assert!(result.is_ok());
            let result = result.unwrap();
            assert!(result.is_some());
            let meta = result.unwrap();
            assert_eq!(meta.total_stored, i + 1);
            assert_eq!(meta.oldest_stored_hash, produced_headers[0].hash());
        }

        let current_size = data.encode().len();
        // Due to underlying DB's design there is slight drift equivalent
        // to amount of headers we are allowed to store.
        let size_drift_allowed = max_headers_allowed_to_store as usize;

        // Adding more header should not increase the size, as we have stored max headers
        for i in max_headers_allowed_to_store
            ..((max_headers_allowed_to_store * 200) + (max_headers_allowed_to_store - 2) + 1)
        {
            current_header = create_next_header(current_header.clone());
            produced_headers.push(current_header.clone());

            assert!(StorageT::<Block>::import_header(
                &storage,
                current_header.clone(),
                NewBlockState::Best
            )
            .is_ok());
            assert!(data.encode().len() <= current_size + size_drift_allowed);

            let last_header_to_be_deleted = i - max_headers_allowed_to_store;

            // Headers at less than or equal to last_header_to_be_deleted won't
            // exists in DB.
            for i in 0..=last_header_to_be_deleted {
                let result = HeaderBackend::<Block>::header(
                    &storage,
                    BlockId::<Block>::Hash(produced_headers[i as usize].hash()),
                );
                assert!(result.is_ok());
                assert!(result.unwrap().is_none());
            }

            // All headers after last_header_to_be_deleted should exists
            for i in (last_header_to_be_deleted + 1)..=i {
                let result = HeaderBackend::<Block>::header(
                    &storage,
                    BlockId::<Block>::Hash(produced_headers[i as usize].hash()),
                );
                assert!(result.is_ok());
                assert!(result.unwrap().is_some());
            }

            // Check if meta is updated correctly.
            let result = storage.fetch_meta::<NumberFor<Block>, <Block as BlockT>::Hash>();
            assert!(result.is_ok());
            let result = result.unwrap();
            assert!(result.is_some());
            let meta = result.unwrap();
            assert_eq!(meta.total_stored, max_headers_allowed_to_store);
            assert_eq!(
                meta.oldest_stored_hash,
                produced_headers[last_header_to_be_deleted as usize + 1].hash()
            );
        }

        // Now, let's check if reducing max_headers_allowed_to_store parameter reduces storage.
        let max_headers_allowed_to_store = max_headers_allowed_to_store - 3;
        let result = Storage::new(data.clone(), max_headers_allowed_to_store);
        assert!(result.is_ok());
        let storage = result.unwrap();
        current_header = create_next_header(current_header.clone());
        produced_headers.push(current_header.clone());
        assert!(StorageT::<Block>::import_header(
            &storage,
            current_header.clone(),
            NewBlockState::Best
        )
        .is_ok());
        assert!(data.encode().len() < current_size);
        // Updating current size and size drift as per new max_headers_allowed_to_store
        // value.
        let current_size = data.encode().len();
        let size_drift_allowed = max_headers_allowed_to_store;
        let result = storage.fetch_meta::<NumberFor<Block>, <Block as BlockT>::Hash>();
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.is_some());
        let meta = result.unwrap();
        assert_eq!(meta.total_stored, max_headers_allowed_to_store);
        // we need to go back max_headers_allowed_to_store - 1 place in the produced headers array.
        assert_eq!(
            meta.oldest_stored_hash,
            produced_headers
                [(produced_headers.len() - 1) - max_headers_allowed_to_store as usize + 1]
                .hash()
        );
        // Last max_headers_allowed_to_store blocks should exists in db
        for i in 0..max_headers_allowed_to_store {
            let result = HeaderBackend::<Block>::header(
                &storage,
                BlockId::<Block>::Hash(
                    produced_headers[produced_headers.len() - 1 - i as usize].hash(),
                ),
            );
            assert!(result.is_ok());
            assert!(result.unwrap().is_some());
        }
        let previous_oldest_stored_hash = meta.oldest_stored_hash;
        let previosuly_stored_blocks = meta.total_stored;

        // Now, let's check if increasing max_headers_allowed_to_store_parameter allows storage to grow
        let max_headers_allowed_to_store = max_headers_allowed_to_store + 3;
        let result = Storage::new(data.clone(), max_headers_allowed_to_store);
        assert!(result.is_ok());
        let storage = result.unwrap();
        current_header = create_next_header(current_header.clone());
        produced_headers.push(current_header.clone());
        assert!(StorageT::<Block>::import_header(
            &storage,
            current_header.clone(),
            NewBlockState::Best
        )
        .is_ok());
        // Now, we are able to increase size beyond our previous size.
        assert!(data.encode().len() > current_size + size_drift_allowed as usize);
        let result = storage.fetch_meta::<NumberFor<Block>, <Block as BlockT>::Hash>();
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.is_some());
        let meta = result.unwrap();
        assert_eq!(meta.total_stored, previosuly_stored_blocks + 1);
        // Oldest stored hash should still be same as previous oldest stored hash as we didn't need to
        // remove anything
        assert_eq!(meta.oldest_stored_hash, previous_oldest_stored_hash);

        // previously stored blocks + 1 blocks should exists in db
        for i in 0..(previosuly_stored_blocks + 1) {
            let result = HeaderBackend::<Block>::header(
                &storage,
                BlockId::<Block>::Hash(
                    produced_headers[produced_headers.len() - 1 - i as usize].hash(),
                ),
            );
            assert!(result.is_ok());
            assert!(result.unwrap().is_some());
        }
    }
}
