use crate::common::traits::aux_store::AuxStore;
use crate::common::traits::header_backend::HeaderBackend;
use crate::common::traits::verifier::Verifier;
use crate::common::types::block_import_params::BlockImportParams;
use crate::common::types::block_origin::BlockOrigin;
use crate::common::types::light_authority_set::LightAuthoritySet;
use crate::common::types::next_change_in_authority::NextChangeInAuthority;
use crate::common::utils::{
    delete_next_authority_change, fetch_light_authority_set, fetch_next_authority_change,
    insert_light_authority_set, GRANDPA_AUTHORITY_CHANGE_INTERMEDIATE_KEY,
};
use parity_scale_codec::alloc::borrow::Cow;
use parity_scale_codec::alloc::sync::Arc;
use sp_finality_grandpa::{ConsensusLog, ScheduledChange, GRANDPA_ENGINE_ID};
use sp_runtime::generic::OpaqueDigestItemId;
use sp_runtime::traits::Header;
use sp_runtime::traits::{Block as BlockT, NumberFor};

fn find_scheduled_change<B: BlockT>(header: &B::Header) -> Option<ScheduledChange<NumberFor<B>>> {
    let id = OpaqueDigestItemId::Consensus(&GRANDPA_ENGINE_ID);

    let filter_log = |log: ConsensusLog<NumberFor<B>>| match log {
        ConsensusLog::ScheduledChange(change) => Some(change),
        _ => None,
    };

    // find the first consensus digest with the right ID which converts to
    // the right kind of consensus log.
    header
        .digest()
        .convert_first(|l| l.try_to(id).and_then(filter_log))
}

pub struct GrandpaVerifier<S> {
    storage: Arc<S>,
}

impl<S> GrandpaVerifier<S> {
    pub fn new(storage: Arc<S>) -> Self {
        Self {
            storage: storage.clone(),
        }
    }
}

impl<S, Block> Verifier<Block> for GrandpaVerifier<S>
where
    S: AuxStore + HeaderBackend<Block>,
    Block: BlockT,
{
    fn verify(
        &mut self,
        _origin: BlockOrigin,
        header: <Block as BlockT>::Header,
        justification: Option<Vec<u8>>,
        _body: Option<Vec<<Block as BlockT>::Extrinsic>>,
    ) -> Result<BlockImportParams<Block>, String> {
        let (possible_authority_change, scheduled_change_exists) = {
            let possible_authority_change =
                fetch_next_authority_change::<S, Block>(self.storage.clone())
                    .map_err(|e| format!("{}", e))?;
            match possible_authority_change {
                Some(authority_change) => {
                    if authority_change.next_change_at == *header.number() {
                        delete_next_authority_change(self.storage.clone())
                            .map_err(|e| format!("{}", e))?;
                        (Some(authority_change), false)
                    } else {
                        (None, true)
                    }
                }
                None => (None, false),
            }
        };

        if let Some(authority_change) = possible_authority_change.as_ref() {
            let (_, enacting_header_number) = authority_change.block_enacting_this_change;
            let info = self.storage.info();
            if info.finalized_number < enacting_header_number {
                return Err("block trying to enact new authority set isn't finalized".into());
            }
        }

        let found_scheduled_authority_change = find_scheduled_change::<Block>(&header);
        let possible_next_authority_change: Option<NextChangeInAuthority<Block>> =
            match found_scheduled_authority_change {
                Some(scheduled_change) => {
                    if scheduled_change_exists {
                        Err("Scheduled change already exists.")
                    } else {
                        Ok(Some(NextChangeInAuthority::new(
                            *header.number() + scheduled_change.delay,
                            (header.hash(), *header.number()),
                            scheduled_change,
                        )))
                    }
                }
                None => Ok(None),
            }?;

        let mut block_import_params: BlockImportParams<Block> =
            BlockImportParams::new(BlockOrigin::NetworkBroadcast, header);
        block_import_params.justification = justification;
        if let Some(next_authority_change) = possible_next_authority_change {
            block_import_params.intermediates.insert(
                Cow::from(GRANDPA_AUTHORITY_CHANGE_INTERMEDIATE_KEY),
                Box::new(next_authority_change),
            );
        }

        if let Some(authority_change) = possible_authority_change {
            let possible_current_authority_set =
                fetch_light_authority_set(self.storage.clone()).map_err(|e| format!("{}", e))?;
            let current_authority_set = if possible_current_authority_set.is_none() {
                Err("No previous authority set found")
            } else {
                Ok(possible_current_authority_set.unwrap())
            }?;
            let next_authority_set = LightAuthoritySet::construct_next_authority_set(
                &current_authority_set,
                authority_change.change.next_authorities,
            );
            insert_light_authority_set(self.storage.clone(), next_authority_set)
                .map_err(|e| format!("{}", e))?;
        }

        Ok(block_import_params)
    }
}
