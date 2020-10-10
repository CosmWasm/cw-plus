use std::collections::{HashMap, HashSet};

use finality_grandpa::voter_set::VoterSet;
use finality_grandpa::{BlockNumberOps, Error as GrandpaError};
use parity_scale_codec::{Decode, Encode};
use sp_core::crypto::Pair;
use sp_finality_grandpa::{
    AuthorityId, AuthorityPair, AuthoritySignature, RoundNumber, SetId as SetIdNumber,
};
use sp_runtime::traits::{Block as BlockT, Header as HeaderT, NumberFor};
use sp_runtime::Justification;

use crate::common::types::blockchain_error::BlockchainError;
use crate::common::types::blockchain_result::BlockchainResult;

/// Errors that can occur while voting in GRANDPA.
#[derive(Debug)]
pub enum Error {
    /// An error within grandpa.
    Grandpa(GrandpaError),
    /// Could not complete a round on disk.
    Client(BlockchainError),
}

impl From<GrandpaError> for Error {
    fn from(e: GrandpaError) -> Self {
        Error::Grandpa(e)
    }
}

impl From<BlockchainError> for Error {
    fn from(e: BlockchainError) -> Self {
        Error::Client(e)
    }
}

/// A GRANDPA message for a substrate chain.
pub type Message<Block> = finality_grandpa::Message<<Block as BlockT>::Hash, NumberFor<Block>>;
pub type Precommit<Block> = finality_grandpa::Precommit<<Block as BlockT>::Hash, NumberFor<Block>>;

/// Justification used to prove block finality.
pub trait ProvableJustification<Block: BlockT>: Encode + Decode {
    /// Verify justification with respect to authorities set and authorities set id.
    fn verify(&self, set_id: u64, authorities: &[(AuthorityId, u64)]) -> BlockchainResult<()>;

    /// Verify justification as well as check if it is targeting correct block
    fn verify_finalization(
        &self,
        set_id: u64,
        finalized_target: (Block::Hash, NumberFor<Block>),
        authorities: &[(AuthorityId, u64)],
    ) -> BlockchainResult<()>;

    fn decode_and_verify_finalization(
        justification: &Justification,
        set_id: u64,
        finalized_target: (Block::Hash, NumberFor<Block>),
        authorities: &[(AuthorityId, u64)],
    ) -> BlockchainResult<()> {
        let justification = Self::decode(&mut &**justification)
            .map_err(|_| BlockchainError::JustificationDecode)?;
        justification.verify_finalization(set_id, finalized_target, authorities)
    }
}

/// Check a message signature by encoding the message as a localized payload and
/// verifying the provided signature using the expected authority id.
/// The encoding necessary to verify the signature will be done using the given
/// buffer, the original content of the buffer will be cleared.
pub fn check_message_sig_with_buffer<Block: BlockT>(
    message: &Message<Block>,
    id: &AuthorityId,
    signature: &AuthoritySignature,
    round: RoundNumber,
    set_id: SetIdNumber,
    buf: &mut Vec<u8>,
) -> Result<(), ()> {
    let as_public = id.clone();
    localized_payload_with_buffer(round, set_id, message, buf);

    if AuthorityPair::verify(signature, buf, &as_public) {
        Ok(())
    } else {
        Err(())
    }
}

/// Encode round message localized to a given round and set id using the given
/// buffer. The given buffer will be cleared and the resulting encoded payload
/// will always be written to the start of the buffer.
pub(crate) fn localized_payload_with_buffer<E: Encode>(
    round: RoundNumber,
    set_id: SetIdNumber,
    message: &E,
    buf: &mut Vec<u8>,
) {
    buf.clear();
    (message, round, set_id).encode_to(buf)
}

/// A commit message for this chain's block type.
pub type Commit<Block> = finality_grandpa::Commit<
    <Block as BlockT>::Hash,
    NumberFor<Block>,
    AuthoritySignature,
    AuthorityId,
>;

/// A GRANDPA justification for block finality, it includes a commit message and
/// an ancestry proof including all headers routing all precommit target blocks
/// to the commit target block. Due to the current voting strategy the precommit
/// targets should be the same as the commit target, since honest voters don't
/// vote past authority set change blocks.
///
/// This is meant to be stored in the db and passed around the network to other
/// nodes, and are used by syncing nodes to prove authority set handoffs.
#[derive(Encode, Decode)]
pub struct GrandpaJustification<Block: BlockT> {
    pub round: u64,
    pub commit: Commit<Block>,
    pub votes_ancestries: Vec<Block::Header>,
}

impl<Block: BlockT> GrandpaJustification<Block> {
    /// Validate the commit and the votes'
    /// ancestry proofs finalize the given block.
    pub fn verify_finalization(
        &self,
        set_id: u64,
        finalized_target: (Block::Hash, NumberFor<Block>),
        voters: &VoterSet<AuthorityId>,
    ) -> Result<(), BlockchainError>
    where
        NumberFor<Block>: finality_grandpa::BlockNumberOps,
    {
        if (self.commit.target_hash, self.commit.target_number) != finalized_target {
            let msg = "invalid commit target in grandpa justification".to_string();
            Err(BlockchainError::BadJustification(msg))
        } else {
            self.verify(set_id, voters)
        }
    }

    /// Validate the commit and the votes' ancestry proofs.
    pub fn verify(&self, set_id: u64, voters: &VoterSet<AuthorityId>) -> Result<(), BlockchainError>
    where
        NumberFor<Block>: finality_grandpa::BlockNumberOps,
    {
        use finality_grandpa::Chain;

        let ancestry_chain = AncestryChain::<Block>::new(&self.votes_ancestries);

        match finality_grandpa::validate_commit(&self.commit, voters, &ancestry_chain) {
            Ok(ref result) if result.ghost().is_some() => {}
            _ => {
                let msg = "invalid commit in grandpa justification".to_string();
                return Err(BlockchainError::BadJustification(msg));
            }
        }

        let mut buf = Vec::new();
        let mut visited_hashes = HashSet::new();
        for signed in self.commit.precommits.iter() {
            if let Err(_) = check_message_sig_with_buffer::<Block>(
                &finality_grandpa::Message::Precommit(signed.precommit.clone()),
                &signed.id,
                &signed.signature,
                self.round,
                set_id,
                &mut buf,
            ) {
                return Err(BlockchainError::BadJustification(
                    "invalid signature for precommit in grandpa justification".to_string(),
                )
                .into());
            }

            if self.commit.target_hash == signed.precommit.target_hash {
                continue;
            }

            match ancestry_chain.ancestry(self.commit.target_hash, signed.precommit.target_hash) {
                Ok(route) => {
                    // ancestry starts from parent hash but the precommit target hash has been visited
                    visited_hashes.insert(signed.precommit.target_hash);
                    for hash in route {
                        visited_hashes.insert(hash);
                    }
                }
                _ => {
                    return Err(BlockchainError::BadJustification(
                        "invalid precommit ancestry proof in grandpa justification".to_string(),
                    )
                    .into());
                }
            }
        }

        let ancestry_hashes = self
            .votes_ancestries
            .iter()
            .map(|h: &Block::Header| h.hash())
            .collect();

        if visited_hashes != ancestry_hashes {
            return Err(BlockchainError::BadJustification(
                "invalid precommit ancestries in grandpa justification with unused headers"
                    .to_string(),
            )
            .into());
        }

        Ok(())
    }
}

impl<Block: BlockT> ProvableJustification<Block> for GrandpaJustification<Block>
where
    NumberFor<Block>: BlockNumberOps,
{
    fn verify(&self, set_id: u64, authorities: &[(AuthorityId, u64)]) -> BlockchainResult<()> {
        let voter_set = VoterSet::new(authorities.clone().to_owned().drain(..)).unwrap();
        GrandpaJustification::verify(self, set_id, &voter_set)
    }

    fn verify_finalization(
        &self,
        set_id: u64,
        finalized_target: (Block::Hash, NumberFor<Block>),
        authorities: &[(AuthorityId, u64)],
    ) -> BlockchainResult<()> {
        let voter_set = VoterSet::new(authorities.clone().to_owned().drain(..)).unwrap();
        GrandpaJustification::verify_finalization(self, set_id, finalized_target, &voter_set)?;
        Ok(())
    }
}

/// A utility trait implementing `finality_grandpa::Chain` using a given set of headers.
/// This is useful when validating commits, using the given set of headers to
/// verify a valid ancestry route to the target commit block.
struct AncestryChain<Block: BlockT> {
    ancestry: HashMap<Block::Hash, Block::Header>,
}

impl<Block: BlockT> AncestryChain<Block> {
    fn new(ancestry: &[Block::Header]) -> AncestryChain<Block> {
        let ancestry: HashMap<_, _> = ancestry
            .iter()
            .cloned()
            .map(|h: Block::Header| (h.hash(), h))
            .collect();

        AncestryChain { ancestry }
    }
}

impl<Block: BlockT> finality_grandpa::Chain<Block::Hash, NumberFor<Block>> for AncestryChain<Block>
where
    NumberFor<Block>: finality_grandpa::BlockNumberOps,
{
    fn ancestry(
        &self,
        base: Block::Hash,
        block: Block::Hash,
    ) -> Result<Vec<Block::Hash>, GrandpaError> {
        let mut route = Vec::new();
        let mut current_hash = block;
        loop {
            if current_hash == base {
                break;
            }
            match self.ancestry.get(&current_hash) {
                Some(current_header) => {
                    current_hash = *current_header.parent_hash();
                    route.push(current_hash);
                }
                _ => return Err(GrandpaError::NotDescendent),
            }
        }
        route.pop(); // remove the base

        Ok(route)
    }

    fn best_chain_containing(
        &self,
        _block: Block::Hash,
    ) -> Option<(Block::Hash, NumberFor<Block>)> {
        None
    }
}
