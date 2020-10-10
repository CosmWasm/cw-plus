use crate::common::traits::aux_store::AuxStore;
use crate::common::types::blockchain_error::BlockchainError;
use crate::common::types::light_authority_set::LightAuthoritySet;
use crate::common::types::next_change_in_authority::NextChangeInAuthority;
use crate::db;
use crate::storage::Storage;
use parity_scale_codec::alloc::sync::Arc;
use parity_scale_codec::{Decode, Encode};
use sp_runtime::traits::Block as BlockT;

// Purposely shorthanded name just to save few bytes of storage
pub const NEXT_CHANGE_IN_AUTHORITY_KEY: &'static [u8] = b"nca";
pub static GRANDPA_AUTHORITY_CHANGE_INTERMEDIATE_KEY: &[u8] = b"grandpa_aci";

/// LightAuthoritySet is saved under this key in aux storage.
pub const LIGHT_AUTHORITY_SET_KEY: &[u8] = b"grandpa_voters";

// Columns supported in our in memory db
pub const NUM_COLUMNS: u32 = 11;

pub fn initialize_storage(
    encoded_data: Vec<u8>,
    max_headers_allowed_to_store: u64,
) -> Result<(db::Data, Arc<Storage>), BlockchainError> {
    let data = db::Data::decode(&mut encoded_data.as_slice()).unwrap();

    return Ok((
        data.clone(),
        Arc::new(Storage::new(data, max_headers_allowed_to_store)?),
    ));
}

pub fn store_next_authority_change<AS, Block>(
    aux_store: Arc<AS>,
    next_authority_change: &NextChangeInAuthority<Block>,
) -> Result<(), BlockchainError>
where
    AS: AuxStore,
    Block: BlockT,
{
    aux_store.insert_aux(
        &[(
            NEXT_CHANGE_IN_AUTHORITY_KEY,
            next_authority_change.encode().as_slice(),
        )],
        &[],
    )
}

pub fn delete_next_authority_change<AS>(aux_store: Arc<AS>) -> Result<(), BlockchainError>
where
    AS: AuxStore,
{
    aux_store.insert_aux(&[], &[NEXT_CHANGE_IN_AUTHORITY_KEY])
}

pub fn fetch_next_authority_change<AS, Block>(
    aux_store: Arc<AS>,
) -> Result<Option<NextChangeInAuthority<Block>>, BlockchainError>
where
    AS: AuxStore,
    Block: BlockT,
{
    let encoded_next_possible_authority_change = aux_store.get_aux(NEXT_CHANGE_IN_AUTHORITY_KEY)?;

    if encoded_next_possible_authority_change.is_none() {
        return Ok(None);
    }

    let encoded_authority_change = encoded_next_possible_authority_change.unwrap();

    let next_change_in_authority: NextChangeInAuthority<Block> =
        NextChangeInAuthority::decode(&mut encoded_authority_change.as_slice()).map_err(|err| {
            BlockchainError::Backend(format!(
                "Unable to decode next change in authority. DB might be corrupted. Underlying Error: {}",
                err.what()
            ))
        })?;

    Ok(Some(next_change_in_authority))
}

pub fn insert_light_authority_set<AS>(
    aux_store: Arc<AS>,
    light_authority_set: LightAuthoritySet,
) -> Result<(), BlockchainError>
where
    AS: AuxStore,
{
    aux_store.insert_aux(
        &[(
            LIGHT_AUTHORITY_SET_KEY,
            light_authority_set.encode().as_slice(),
        )],
        &[],
    )
}

pub fn fetch_light_authority_set<AS>(
    aux_store: Arc<AS>,
) -> Result<Option<LightAuthoritySet>, BlockchainError>
where
    AS: AuxStore,
{
    let encoded_possible_light_authority_set = aux_store.get_aux(LIGHT_AUTHORITY_SET_KEY)?;

    if encoded_possible_light_authority_set.is_none() {
        return Ok(None);
    }

    let encoded_light_authority_set = encoded_possible_light_authority_set.unwrap();

    let light_authority_set =
        LightAuthoritySet::decode(&mut encoded_light_authority_set.as_slice()).map_err(|err| {
            BlockchainError::Backend(format!(
                "Unable to decode light authority set. DB might be corrupted. Underlying Error: {}",
                err.what()
            ))
        })?;

    Ok(Some(light_authority_set))
}
