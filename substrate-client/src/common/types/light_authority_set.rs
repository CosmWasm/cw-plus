use parity_scale_codec::{Decode, Encode};
use sp_finality_grandpa::AuthorityList;

/// Latest authority set tracker.
#[derive(Debug, Encode, Decode, Clone, Default)]
pub struct LightAuthoritySet {
    set_id: u64,
    authorities: AuthorityList,
}

impl LightAuthoritySet {
    pub fn new(set_id: u64, authorities: AuthorityList) -> Self {
        Self {
            set_id,
            authorities,
        }
    }

    pub fn construct_next_authority_set(
        prev_authority_set: &LightAuthoritySet,
        new_authority_list: AuthorityList,
    ) -> Self {
        Self {
            set_id: prev_authority_set.set_id + 1,
            authorities: new_authority_list,
        }
    }

    pub fn set_id(&self) -> u64 {
        self.set_id
    }

    pub fn authorities(&self) -> AuthorityList {
        self.authorities.clone()
    }
}
