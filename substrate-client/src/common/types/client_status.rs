use crate::common::types::light_authority_set::LightAuthoritySet;
use crate::common::types::next_change_in_authority::NextChangeInAuthority;
use sp_runtime::traits::Block as BlockT;

pub struct ClientStatus<Block>
where
    Block: BlockT,
{
    pub possible_last_finalized_header: Option<Block::Header>,
    pub possible_light_authority_set: Option<LightAuthoritySet>,
    pub possible_next_change_in_authority: Option<NextChangeInAuthority<Block>>,
    pub possible_best_header: Option<Block::Header>,
}
