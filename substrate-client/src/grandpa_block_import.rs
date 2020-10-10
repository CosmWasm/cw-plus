use crate::common::traits::block_import::BlockImport;
use crate::common::traits::finalizer::Finalizer;
use crate::common::traits::header_backend::HeaderBackend;
use crate::common::traits::storage::Storage;
use crate::common::types::block_check_params::BlockCheckParams;
use crate::common::types::block_import_params::BlockImportParams;
use crate::common::types::blockchain_error::BlockchainError;
use crate::common::types::consensus_error::ConsensusError;
use crate::common::types::import_result::ImportResult;
use crate::common::types::imported_aux::ImportedAux;
use crate::common::utils::fetch_light_authority_set;
use crate::justification::{GrandpaJustification, ProvableJustification};
use finality_grandpa::BlockNumberOps;
use parity_scale_codec::{Decode, Encode};
use sp_api::BlockId;
use sp_finality_grandpa::AuthorityList;
use sp_runtime::traits::{Block as BlockT, DigestFor, Header, NumberFor};
use sp_runtime::Justification;
use std::sync::Arc;

/// Latest authority set tracker.
#[derive(Debug, Encode, Decode)]
struct LightAuthoritySet {
    set_id: u64,
    authorities: AuthorityList,
}

/// A light block-import handler for GRANDPA.
///
/// It is responsible for:
/// - checking GRANDPA justifications;
/// - fetching finality proofs for blocks that are enacting consensus changes.
pub struct GrandpaLightBlockImport<Client, S> {
    client: Arc<Client>,
    storage: Arc<S>,
}

impl<Client, S> GrandpaLightBlockImport<Client, S> {
    pub fn new(client: Arc<Client>, storage: Arc<S>) -> Self {
        Self {
            client: client.clone(),
            storage: storage.clone(),
        }
    }
}

impl<Client, S> Clone for GrandpaLightBlockImport<Client, S> {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            storage: self.storage.clone(),
        }
    }
}

impl<S, Block: BlockT, Client> BlockImport<Block> for GrandpaLightBlockImport<Client, S>
where
    NumberFor<Block>: BlockNumberOps,
    DigestFor<Block>: Encode,
    S: Storage<Block>,
    for<'a> &'a Client:
        HeaderBackend<Block> + BlockImport<Block, Error = ConsensusError> + Finalizer<Block>,
{
    type Error = ConsensusError;

    fn check_block(&mut self, block: BlockCheckParams<Block>) -> Result<ImportResult, Self::Error> {
        self.client.as_ref().check_block(block)
    }

    fn import_block(
        &mut self,
        block: BlockImportParams<Block>,
    ) -> Result<ImportResult, Self::Error> {
        do_import_block::<_, _, _, GrandpaJustification<Block>>(
            &*self.client,
            self.storage.clone(),
            block,
        )
    }
}

/// Try to import new block.
fn do_import_block<S, C, Block: BlockT, J>(
    mut client: C,
    storage: Arc<S>,
    mut block: BlockImportParams<Block>,
) -> Result<ImportResult, ConsensusError>
where
    C: HeaderBackend<Block> + Finalizer<Block> + BlockImport<Block> + Clone,
    S: Storage<Block>,
    NumberFor<Block>: finality_grandpa::BlockNumberOps,
    DigestFor<Block>: Encode,
    J: ProvableJustification<Block>,
{
    let hash = block.header.hash();
    let number = block.header.number().clone();

    // we don't want to finalize on `inner.import_block`
    let justification = block.justification.take();
    let import_result = client.import_block(block);

    let imported_aux = match import_result {
        Ok(ImportResult::Imported(aux)) => aux,
        Ok(r) => return Ok(r),
        Err(e) => return Err(ConsensusError::ClientImport(e.to_string()).into()),
    };

    match justification {
        Some(justification) => {
            do_import_justification::<_, _, _, J>(client, storage, hash, number, justification)
        }
        None => Ok(ImportResult::Imported(imported_aux)),
    }
}

/// Try to import justification.
fn do_import_justification<S, C, Block: BlockT, J>(
    client: C,
    storage: Arc<S>,
    hash: Block::Hash,
    number: NumberFor<Block>,
    justification: Justification,
) -> Result<ImportResult, ConsensusError>
where
    C: HeaderBackend<Block> + Finalizer<Block> + Clone,
    S: Storage<Block>,
    NumberFor<Block>: finality_grandpa::BlockNumberOps,
    J: ProvableJustification<Block>,
{
    let possible_light_authority_set =
        fetch_light_authority_set(storage).map_err(|e| ConsensusError::Other(Box::new(e)))?;
    if possible_light_authority_set.is_none() {
        return Err(ConsensusError::InvalidAuthoritiesSet);
    }
    let light_authority_set = possible_light_authority_set.unwrap();

    // Verify if justification is valid and it finalizes correct block
    let justification = J::decode_and_verify_finalization(
        &justification,
        light_authority_set.set_id(),
        (hash, number),
        &light_authority_set.authorities(),
    );

    // BadJustification error means that justification has been successfully decoded, but
    // it isn't valid within current authority set
    let justification = match justification {
        Err(BlockchainError::BadJustification(_)) => {
            let mut imported_aux = ImportedAux::default();
            imported_aux.needs_finality_proof = true;
            return Ok(ImportResult::Imported(imported_aux));
        }
        Err(e) => {
            return Err(ConsensusError::ClientImport(e.to_string()).into());
        }
        Ok(justification) => justification,
    };

    // finalize the block
    do_finalize_block(client, hash, number, justification.encode())
}

/// Finalize the block.
fn do_finalize_block<C, Block: BlockT>(
    client: C,
    hash: Block::Hash,
    _number: NumberFor<Block>,
    justification: Justification,
) -> Result<ImportResult, ConsensusError>
where
    C: HeaderBackend<Block> + Finalizer<Block> + Clone,
    NumberFor<Block>: finality_grandpa::BlockNumberOps,
{
    // finalize the block
    client
        .finalize_block(BlockId::Hash(hash), Some(justification))
        .map_err(|e| ConsensusError::ClientImport(e.to_string()))?;

    // we just finalized this block, so if we were importing it, it is now the new best
    Ok(ImportResult::imported(true))
}
