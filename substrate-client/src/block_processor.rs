use crate::block_import_wrapper::BlockImportWrapper;
use crate::client::Client;
use crate::common::traits::block_import::BlockImport;
use crate::common::traits::verifier::Verifier;
use crate::common::types::block_check_params::BlockCheckParams;
use crate::common::types::block_import_error::BlockImportError;
use crate::common::types::block_import_result::BlockImportResult;
use crate::common::types::block_origin::BlockOrigin;
use crate::common::types::blockchain_result::BlockchainResult;
use crate::common::types::consensus_error::ConsensusError;
use crate::common::types::import_result::ImportResult;
use crate::common::types::incoming_block::IncomingBlock;
use crate::common::utils::initialize_storage;
use crate::db;
use crate::grandpa_block_import::GrandpaLightBlockImport;
use crate::types::Block;
use crate::verifier::GrandpaVerifier;
use sp_runtime::traits::{Block as BlockT, Header, NumberFor};
use std::sync::Arc;

pub type BlockProcessor<B> =
    Box<dyn FnMut(IncomingBlock<B>) -> Result<BlockImportResult<NumberFor<B>>, String>>;

pub fn setup_block_processor(
    encoded_data: Vec<u8>,
    max_non_finalized_blocks_allowed: u64,
) -> BlockchainResult<(BlockProcessor<Block>, db::Data)> {
    let (data, storage) = initialize_storage(encoded_data, max_non_finalized_blocks_allowed)?;

    // Custom client implementation with dummy runtime
    let client = Arc::new(Client::new(storage.clone()));

    // We need to re-initialize grandpa light import queue because
    // current version read/write authority set from private field instead of
    // auxiliary storage.
    let block_processor_fn = Box::new(move |incoming_block: IncomingBlock<Block>| {
        let grandpa_block_import = GrandpaLightBlockImport::new(client.clone(), storage.clone());
        let mut grandpa_verifier = GrandpaVerifier::new(storage.clone());
        let mut block_import_wrapper: BlockImportWrapper<_, _> =
            BlockImportWrapper::new(grandpa_block_import.clone(), storage.clone());
        import_single_block(
            &mut block_import_wrapper,
            BlockOrigin::NetworkBroadcast,
            incoming_block,
            &mut grandpa_verifier,
        )
        .map_err(|e| format!("{:?}", e))
    });

    Ok((block_processor_fn, data))
}

/// Single block import function.
fn import_single_block<B: BlockT, V: Verifier<B>>(
    import_handle: &mut dyn BlockImport<B, Error = ConsensusError>,
    block_origin: BlockOrigin,
    block: IncomingBlock<B>,
    verifier: &mut V,
) -> Result<BlockImportResult<NumberFor<B>>, BlockImportError> {
    let (header, justification) = match (block.header, block.justification) {
        (Some(header), justification) => (header, justification),
        (None, _) => {
            return Err(BlockImportError::IncompleteHeader);
        }
    };

    let number = header.number().clone();
    let hash = header.hash();
    let parent_hash = header.parent_hash().clone();

    let import_error = |e| match e {
        Ok(ImportResult::AlreadyInChain) => Ok(BlockImportResult::ImportedKnown(number)),
        Ok(ImportResult::Imported(aux)) => Ok(BlockImportResult::ImportedUnknown(number, aux)),
        Ok(ImportResult::MissingState) => Err(BlockImportError::MissingState),
        Ok(ImportResult::UnknownParent) => Err(BlockImportError::UnknownParent),
        Ok(ImportResult::KnownBad) => Err(BlockImportError::BadBlock),
        Err(e) => Err(BlockImportError::Other(e)),
    };
    match import_error(import_handle.check_block(BlockCheckParams {
        hash,
        number,
        parent_hash,
        allow_missing_state: block.allow_missing_state,
        import_existing: block.import_existing,
    }))? {
        BlockImportResult::ImportedUnknown { .. } => (),
        r => return Ok(r), // Any other successful result means that the block is already imported.
    }

    let mut import_block = verifier
        .verify(block_origin, header, justification, block.body)
        .map_err(|msg| BlockImportError::VerificationFailed(msg))?;

    import_block.allow_missing_state = block.allow_missing_state;

    import_error(import_handle.import_block(import_block))
}
