use crate::block_processor::setup_block_processor;
use crate::common::traits::header_backend::HeaderBackend;
use crate::common::traits::storage::Storage as StorageT;
use crate::common::types::block_import_result::BlockImportResult;
use crate::common::types::blockchain_error::BlockchainError;
use crate::common::types::blockchain_info::BlockchainInfo;
use crate::common::types::client_status::ClientStatus;
use crate::common::types::incoming_block::IncomingBlock;
use crate::common::types::light_authority_set::LightAuthoritySet;
use crate::common::types::new_block_state::NewBlockState;
use crate::common::utils::{
    fetch_light_authority_set, fetch_next_authority_change, initialize_storage,
    insert_light_authority_set, NUM_COLUMNS,
};
use crate::db::create;
use crate::genesis::GenesisData;
use crate::types::{Block, Header};
use parity_scale_codec::Encode;
use sp_api::BlockId;
use sp_runtime::traits::{Block as BlockT, NumberFor};
use sp_runtime::Justification;

/// Initializes the database with initial header
/// and authority set
pub(crate) fn initialize_state(
    initial_header: Header,
    initial_authority_set: LightAuthoritySet,
    max_headers_allowed_to_store: u64,
) -> Result<Vec<u8>, BlockchainError> {
    let db = create(NUM_COLUMNS);
    let new_data = crate::db::Data {
        db,
        genesis_data: GenesisData {},
    };
    let empty_data = new_data.encode();
    let (data, storage) = initialize_storage(empty_data, max_headers_allowed_to_store)?;
    insert_light_authority_set(storage.clone(), initial_authority_set)?;
    StorageT::<Block>::import_header(storage.as_ref(), initial_header, NewBlockState::Best)?;

    Ok(data.encode())
}

/// Gives current status of database passed which includes
/// current best header, finalized header, light authority set
/// as well as next authority set change scheduled.
pub(crate) fn current_status<Block>(
    encoded_data: Vec<u8>,
) -> Result<ClientStatus<Block>, BlockchainError>
where
    Block: BlockT,
{
    // It doesn't matter what is the value of max_headers_allowed_to_store as we are only reading the storage meta
    let (_, storage) = initialize_storage(encoded_data, 2)?;
    let possible_light_authority_set = fetch_light_authority_set(storage.clone())?;
    let mut possible_finalized_header: Option<Block::Header> = None;
    let mut possible_best_header: Option<Block::Header> = None;
    let info: BlockchainInfo<Block> = storage.info();
    if info.finalized_hash != Default::default() {
        possible_finalized_header = storage.header(BlockId::<Block>::Hash(info.finalized_hash))?;
    }
    if info.best_hash != Default::default() {
        possible_best_header = storage.header(BlockId::<Block>::Hash(info.best_hash))?;
    }
    let possible_next_change_in_authority = fetch_next_authority_change(storage.clone())?;

    Ok(ClientStatus {
        possible_last_finalized_header: possible_finalized_header,
        possible_light_authority_set,
        possible_next_change_in_authority,
        possible_best_header,
    })
}

/// Ingests finalized header and optionally a justification
/// Until justification is not provided block won't be marked as
/// finalized. And if there are already `max_non_finalized_blocks`
/// in db, it won't accept another header.
pub(crate) fn ingest_finalized_header(
    encoded_data: Vec<u8>,
    finalized_header: Header,
    justification: Option<Justification>,
    max_non_finalized_blocks_allowed: u64,
) -> Result<(BlockImportResult<NumberFor<Block>>, Vec<u8>), String> {
    let (mut block_processor_fn, data) =
        setup_block_processor(encoded_data, max_non_finalized_blocks_allowed)
            .map_err(|e| format!("{}", e))?;
    let incoming_block = IncomingBlock {
        hash: finalized_header.hash(),
        header: Some(finalized_header),
        body: None,
        justification,
        allow_missing_state: false,
        import_existing: false,
    };

    // We aren't returning updated db data from block processor function directly, because
    // in future we might want to call it for multiple blocks per tx.
    let block_import_response = block_processor_fn(incoming_block)?;
    match &block_import_response {
        BlockImportResult::ImportedKnown(_) => {}
        BlockImportResult::ImportedUnknown(_, aux) => {
            if aux.bad_justification || aux.needs_finality_proof {
                return Err(format!(
                    "Error: {}",
                    "Justification is invalid or authority set is not updated."
                ));
            }
        }
    }
    Ok((block_import_response, data.encode()))
}

#[cfg(test)]
mod tests {
    use crate::common::types::light_authority_set::LightAuthoritySet;
    use crate::justification::{Commit, GrandpaJustification, Message, Precommit};
    use crate::light_state::{current_status, ingest_finalized_header, initialize_state};
    use crate::types::{Block, Header};
    use clear_on_drop::clear::Clear;
    use finality_grandpa::SignedPrecommit;
    use parity_scale_codec::Encode;
    use sp_core::crypto::Public;
    use sp_core::H256;
    use sp_finality_grandpa::{
        AuthorityId, AuthorityList, AuthoritySignature, ScheduledChange, GRANDPA_ENGINE_ID,
    };
    use sp_keyring::ed25519::Keyring;
    use sp_keyring::Ed25519Keyring;
    use sp_runtime::traits::{Block as BlockT, Header as HeaderT, NumberFor, One};
    use sp_runtime::{DigestItem, Justification};
    use std::io::Write;
    use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

    type PrintLevel = i32;

    fn write_with_print_level(stream: &mut StandardStream, _print_level: PrintLevel, text: String) {
        let mut print_str = String::new();
        // TODO: Enable print levels later on
        print_str.push_str(text.as_str());
        writeln!(stream, "{}", print_str).unwrap();
    }

    fn write_test_flow(text: String) {
        let mut stdout = StandardStream::stdout(ColorChoice::Always);
        stdout
            .set_color(
                ColorSpec::new()
                    .set_fg(Some(Color::Blue))
                    .set_bold(true)
                    .set_italic(false),
            )
            .unwrap();
        write_with_print_level(&mut stdout, 1, text);
        stdout
            .set_color(
                ColorSpec::new()
                    .set_fg(None)
                    .set_bold(false)
                    .set_italic(false),
            )
            .unwrap();
    }

    fn write_success_assert(print_level: PrintLevel, text: String) {
        let mut stdout = StandardStream::stdout(ColorChoice::Always);
        stdout
            .set_color(
                ColorSpec::new()
                    .set_fg(Some(Color::Green))
                    .set_bold(false)
                    .set_italic(true),
            )
            .unwrap();
        write_with_print_level(&mut stdout, print_level, text);
        stdout
            .set_color(
                ColorSpec::new()
                    .set_fg(None)
                    .set_bold(false)
                    .set_italic(false),
            )
            .unwrap();
    }

    fn write_failure_assert(print_level: PrintLevel, text: String) {
        let mut stdout = StandardStream::stdout(ColorChoice::Always);
        stdout
            .set_color(
                ColorSpec::new()
                    .set_fg(Some(Color::Red))
                    .set_bold(false)
                    .set_italic(true),
            )
            .unwrap();
        write_with_print_level(&mut stdout, print_level, text);
        stdout
            .set_color(
                ColorSpec::new()
                    .set_fg(None)
                    .set_bold(false)
                    .set_italic(false),
            )
            .unwrap();
    }

    fn write_neutral_assert(print_level: PrintLevel, text: String) {
        let mut stdout = StandardStream::stdout(ColorChoice::Always);
        stdout
            .set_color(
                ColorSpec::new()
                    .set_fg(None)
                    .set_bold(false)
                    .set_italic(true),
            )
            .unwrap();
        write_with_print_level(&mut stdout, print_level, text);
        stdout
            .set_color(
                ColorSpec::new()
                    .set_fg(None)
                    .set_bold(false)
                    .set_italic(false),
            )
            .unwrap();
    }

    fn write_assert_guards(print_level: PrintLevel, text: String) {
        let mut stdout = StandardStream::stdout(ColorChoice::Always);
        stdout
            .set_color(
                ColorSpec::new()
                    .set_fg(Some(Color::White))
                    .set_bold(true)
                    .set_italic(false),
            )
            .unwrap();
        write_with_print_level(&mut stdout, print_level, text);
        stdout
            .set_color(
                ColorSpec::new()
                    .set_fg(None)
                    .set_bold(false)
                    .set_italic(false),
            )
            .unwrap();
    }

    fn assert_successful_db_init(
        custom_authority_set: Option<LightAuthoritySet>,
        print_level: PrintLevel,
    ) -> (Vec<u8>, Header) {
        let initial_header = Header::new(
            One::one(),
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
        );

        let authority_set = if custom_authority_set.is_some() {
            custom_authority_set.unwrap()
        } else {
            LightAuthoritySet::new(0, vec![])
        };

        write_assert_guards(
            print_level,
            format!("========= Verifying db initialization =========="),
        );
        write_neutral_assert(
            print_level,
            format!(
                "Initializing database with authority set: {:?} and header: {:?}",
                authority_set,
                initial_header.hash()
            ),
        );

        let result = initialize_state(initial_header.clone(), authority_set, 2);
        assert!(result.is_ok());
        let encoded_data = result.unwrap();
        assert!(encoded_data.len() > 0);
        // Best header need to be updated
        internal_assert_best_header(encoded_data.clone(), &initial_header);

        write_success_assert(
            print_level,
            format!(
                "DB initialized and updated best header is: {:?}",
                initial_header.hash()
            ),
        );
        write_assert_guards(
            print_level,
            format!("======== Verified db initialization ============"),
        );

        (encoded_data, initial_header)
    }

    fn assert_successful_header_ingestion(
        encoded_data: Vec<u8>,
        header: Header,
        justification: Option<Justification>,
        print_level: PrintLevel,
    ) -> Vec<u8> {
        write_assert_guards(
            print_level,
            format!("========= Verifying header ingestion =========="),
        );
        write_neutral_assert(
            print_level,
            format!(
                "Ingesting header: {:?} with justification: {:?}",
                header.hash(),
                justification
            ),
        );

        let result = ingest_finalized_header(encoded_data, header.clone(), justification, 256);
        assert!(result.is_ok());
        let encoded_data = result.unwrap().1;
        // Best header need to be updated
        internal_assert_best_header(encoded_data.clone(), &header);

        write_success_assert(
            print_level,
            format!(
                "Header is ingested and updated best header is: {:?}",
                header.hash()
            ),
        );
        write_assert_guards(
            print_level,
            format!("========= Verified header ingestion =========="),
        );

        encoded_data
    }

    fn assert_failed_header_ingestion(
        encoded_data: Vec<u8>,
        header: Header,
        justification: Option<Justification>,
        expected_error: String,
        print_level: PrintLevel,
    ) {
        write_assert_guards(
            print_level,
            format!("========= Verifying header ingestion =========="),
        );
        write_neutral_assert(
            print_level,
            format!(
                "Ingesting header: {:?} with justification: {:?}",
                header.hash(),
                justification
            ),
        );

        let result = ingest_finalized_header(encoded_data, header.clone(), justification, 256);
        assert!(result.is_err());
        assert_eq!(result.err().unwrap(), expected_error);

        write_failure_assert(
            print_level,
            format!(
                "Header ingestion is failed with error: {:?}",
                expected_error
            ),
        );
        write_assert_guards(
            print_level,
            format!("========= Verified header ingestion =========="),
        );
    }

    fn create_next_header(header: Header) -> Header {
        let mut next_header = header.clone();
        next_header.number += 1;
        next_header.parent_hash = header.hash();
        next_header.digest.clear();
        next_header
    }

    fn internal_assert_best_header(encoded_data: Vec<u8>, expected_to_be_best_header: &Header) {
        let result = current_status::<Block>(encoded_data.clone());
        assert!(result.is_ok());
        let status = result.unwrap();
        assert!(status.possible_best_header.is_some());
        let current_best_header = status.possible_best_header.unwrap();
        assert_eq!(&current_best_header, expected_to_be_best_header);
    }

    fn assert_finalized_header(
        encoded_data: Vec<u8>,
        expected_to_be_finalized: &Header,
        print_level: PrintLevel,
    ) {
        write_assert_guards(
            print_level,
            format!("========= Verifying finalized header value =========="),
        );
        write_neutral_assert(
            print_level,
            format!(
                "Checking if finalized header is updated to {:?}",
                expected_to_be_finalized.hash()
            ),
        );

        let result = current_status::<Block>(encoded_data.clone());
        assert!(result.is_ok());
        let status = result.unwrap();
        assert!(status.possible_last_finalized_header.is_some());
        let current_finalized_header = status.possible_last_finalized_header.unwrap();
        assert_eq!(&current_finalized_header, expected_to_be_finalized);

        write_success_assert(
            print_level,
            format!(
                "Finalized header is updated to {:?}",
                current_finalized_header.hash()
            ),
        );
        write_assert_guards(
            print_level,
            format!("========= Verified finalized header value =========="),
        );
    }

    fn assert_authority_set(
        encoded_data: Vec<u8>,
        expected_light_authority_set: &LightAuthoritySet,
        print_level: PrintLevel,
    ) {
        write_assert_guards(
            print_level,
            format!("========= Verifying authority set value =========="),
        );
        write_neutral_assert(
            print_level,
            format!(
                "Checking if light authority set is updated to {:?}",
                expected_light_authority_set
            ),
        );

        let result = current_status::<Block>(encoded_data.clone());
        assert!(result.is_ok());
        let status = result.unwrap();
        assert!(status.possible_light_authority_set.is_some());
        let light_authority_set = status.possible_light_authority_set.unwrap();
        assert_eq!(
            light_authority_set.set_id(),
            expected_light_authority_set.set_id()
        );
        assert_eq!(
            light_authority_set.authorities(),
            expected_light_authority_set.authorities()
        );

        write_success_assert(
            print_level,
            format!(
                "Light authority set is updated to {:?}",
                light_authority_set
            ),
        );
        write_assert_guards(
            print_level,
            format!("========= Verified authority set value =========="),
        );
    }

    fn assert_next_change_in_authority(
        encoded_data: Vec<u8>,
        expected_scheduled_change: &ScheduledChange<NumberFor<Block>>,
        print_level: PrintLevel,
    ) {
        write_assert_guards(
            print_level,
            format!("========= Verifying existence of next change of authority =========="),
        );
        write_neutral_assert(
            print_level,
            format!(
                "Checking if scheduled change is updated to {:?}",
                expected_scheduled_change
            ),
        );

        let result = current_status::<Block>(encoded_data.clone());
        assert!(result.is_ok());
        let status = result.unwrap();
        assert!(status.possible_next_change_in_authority.is_some());
        let scheduled_change = status.possible_next_change_in_authority.unwrap().change;
        assert_eq!(&scheduled_change, expected_scheduled_change);

        write_success_assert(
            print_level,
            format!("Scheduled change is updated to {:?}", scheduled_change),
        );
        write_assert_guards(
            print_level,
            format!("========= Verified existence of next change of authority =========="),
        );
    }

    fn assert_no_next_change_in_authority(encoded_data: Vec<u8>, print_level: PrintLevel) {
        write_assert_guards(
            print_level,
            format!("========= Verifying absence of next change of authority =========="),
        );

        let result = current_status::<Block>(encoded_data.clone());
        assert!(result.is_ok());
        let status = result.unwrap();
        assert!(status.possible_next_change_in_authority.is_none());

        write_success_assert(
            print_level,
            format!("Verified that scheduled change does not exists"),
        );
        write_assert_guards(
            print_level,
            format!("=========================================="),
        );
    }

    #[test]
    fn test_initialize_db_success() {
        let (encoded_data, initial_header) = assert_successful_db_init(None, 1);
        let next_header = create_next_header(initial_header);
        assert_successful_header_ingestion(encoded_data, next_header, None, 1);
    }

    #[test]
    fn test_initialize_db_non_sequential_block() {
        let (encoded_data, initial_header) = assert_successful_db_init(None, 1);

        let mut next_header = create_next_header(initial_header);
        // Let's change number of block to be non sequential
        next_header.number += 1;

        assert_failed_header_ingestion(encoded_data, next_header, None, String::from("Other(ClientImport(\"Import failed: Trying to import blocks in non-sequential order. to be imported block need to be child of last best block or first block itself. Expected block number: 2. Got: 3\"))"), 1);
    }

    #[test]
    fn test_initialize_db_wrong_parent_hash() {
        let (encoded_data, initial_header) = assert_successful_db_init(None, 1);

        let mut next_header = create_next_header(initial_header);
        // Setting wrong parent hash
        next_header.parent_hash = Default::default();

        assert_failed_header_ingestion(
            encoded_data,
            next_header,
            None,
            String::from("UnknownParent"),
            1,
        );
    }

    #[test]
    fn test_authority_set_processing() {
        let genesis_peers = [Ed25519Keyring::Alice, Ed25519Keyring::Bob];
        let genesis_voters = make_ids(&genesis_peers);
        let genesis_authority_set = LightAuthoritySet::new(0, genesis_voters.clone());

        let first_peers = [Ed25519Keyring::Charlie, Ed25519Keyring::Dave];
        let first_voters = make_ids(&first_peers);
        let first_authority_set = LightAuthoritySet::construct_next_authority_set(
            &genesis_authority_set,
            first_voters.clone(),
        );

        let second_peers = [Ed25519Keyring::Eve, Ed25519Keyring::Ferdie];
        let second_voters = make_ids(&second_peers);
        let second_authority_set = LightAuthoritySet::construct_next_authority_set(
            &first_authority_set,
            second_voters.clone(),
        );

        write_test_flow(format!("Starting Authority set processing test"));
        let (encoded_data, initial_header) =
            assert_successful_db_init(Some(genesis_authority_set.clone()), 1);

        let mut first_header = create_next_header(initial_header);
        let change = ScheduledChange {
            next_authorities: first_voters.clone(),
            delay: 3,
        };
        first_header.digest_mut().push(DigestItem::Consensus(
            GRANDPA_ENGINE_ID,
            sp_finality_grandpa::ConsensusLog::ScheduledChange(change.clone()).encode(),
        ));
        let mut second_header = create_next_header(first_header.clone());
        let third_header = create_next_header(second_header.clone());
        let mut fourth_header = create_next_header(third_header.clone());
        let new_change = ScheduledChange {
            next_authorities: second_voters.clone(),
            delay: 2,
        };
        fourth_header.digest_mut().push(DigestItem::Consensus(
            GRANDPA_ENGINE_ID,
            sp_finality_grandpa::ConsensusLog::ScheduledChange(new_change.clone()).encode(),
        ));

        write_test_flow(format!(
            "\n\nPushing scheduled change with next header and verifying data."
        ));
        // Updating encoded data
        let encoded_data =
            assert_successful_header_ingestion(encoded_data, first_header.clone(), None, 1);

        write_test_flow(format!(
            "\n\nWe should now have next scheduled change in database"
        ));
        // We should now have next schedule change in database
        assert_next_change_in_authority(encoded_data.clone(), &change, 1);
        // Current authority set remains same
        assert_authority_set(encoded_data.clone(), &genesis_authority_set, 1);

        write_test_flow(format!(
            "\n\nWe cannot push another authority set while previous one exists"
        ));

        second_header.digest_mut().push(DigestItem::Consensus(
            GRANDPA_ENGINE_ID,
            sp_finality_grandpa::ConsensusLog::ScheduledChange(ScheduledChange {
                next_authorities: vec![
                    (AuthorityId::from_slice(&[2; 32]), 4),
                    (AuthorityId::from_slice(&[2; 32]), 4),
                ],
                delay: 4,
            })
            .encode(),
        ));
        assert_failed_header_ingestion(
            encoded_data.clone(),
            second_header.clone(),
            None,
            String::from("VerificationFailed(\"Scheduled change already exists.\")"),
            1,
        );
        // After clearing digest we should be able to ingest header
        write_test_flow(format!(
            "\n\nAfter clearing header's digest, we were able to ingest it"
        ));
        second_header.digest.clear();
        let encoded_data =
            assert_successful_header_ingestion(encoded_data, second_header.clone(), None, 1);
        let saved_encoded_data = encoded_data.clone();

        write_test_flow(format!(
            "\n\nIf we do not submit justification with this block, next block cannot be ingested as, authority set due to enacted in next block cannot be trusted."
        ));

        let encoded_data =
            assert_successful_header_ingestion(encoded_data, third_header.clone(), None, 1);

        write_test_flow(format!(
            "\n\nWe won't be able to ingest this block, as the older block which introduced authority set due to enacted in this block isn't finalized, yet."
        ));

        assert_failed_header_ingestion(
            encoded_data.clone(),
            fourth_header.clone(),
            None,
            String::from(
                "VerificationFailed(\"block trying to enact new authority set isn\\'t finalized\")",
            ),
            1,
        );

        write_test_flow(format!(
            "\n\nLet's rewind our light client state by restoring state two blocks earlier and submit next block with justification, the header will be processed and authority set will be updated."
        ));

        let commit = create_justification_commit(1, 0, vec![third_header.clone()], &genesis_peers);
        let grandpa_justification: GrandpaJustification<Block> = GrandpaJustification {
            round: 1,
            commit,
            votes_ancestries: vec![],
        };
        let encoded_data = assert_successful_header_ingestion(
            saved_encoded_data,
            third_header.clone(),
            Some(grandpa_justification.encode()),
            1,
        );

        write_test_flow(format!("\n\nNow with ingestion of new block, authority set will be changed. We will also use this block to enact another change."));
        let encoded_data =
            assert_successful_header_ingestion(encoded_data, fourth_header.clone(), None, 1);

        write_test_flow(format!("\n\nNow, we have our authority set changed, and older NextChangeInAuthority struct replaced by new change."));

        // Now, we have our authority set changed, and older NextChangeInAuthority struct replaced
        // by new change

        // Previous change has been overwritten by new change
        assert_next_change_in_authority(encoded_data.clone(), &new_change, 1);

        // We now have authority set enacted as per previous change
        // Last authority set had set_id of 0
        // so while ingesting new authority set it
        // was incremented by 1.
        assert_authority_set(encoded_data.clone(), &first_authority_set, 1);

        // Now, a scenario where scheduled change isn't part of digest after two blocks delay
        // In this case new authority set will be enacted and aux entry will be removed

        write_test_flow(format!("\n\nNow, a scenario where scheduled change isn't part of digest after two blocks delay. In this case new authority set will be enacted and aux entry will be removed"));
        let fifth_header = create_next_header(fourth_header.clone());
        let commit = create_justification_commit(1, 1, vec![fifth_header.clone()], &first_peers);
        let grandpa_justification: GrandpaJustification<Block> = GrandpaJustification {
            round: 1,
            commit,
            votes_ancestries: vec![],
        };
        let encoded_data = assert_successful_header_ingestion(
            encoded_data,
            fifth_header.clone(),
            Some(grandpa_justification.encode()),
            1,
        );

        // new change still same
        assert_next_change_in_authority(encoded_data.clone(), &new_change, 1);

        // authority set still same
        // Last authority set had set_id of 0
        // so while ingesting new authority set it
        // was incremented by 1.
        assert_authority_set(encoded_data.clone(), &first_authority_set, 1);

        let sixth_header = create_next_header(fifth_header.clone());
        let encoded_data =
            assert_successful_header_ingestion(encoded_data, sixth_header.clone(), None, 1);

        write_test_flow(format!(
            "\n\nNow NextChangeInAuthority should be removed from db and authority set is changed"
        ));

        // Now NextChangeInAuthority should be removed from db and authority set is changed
        assert_no_next_change_in_authority(encoded_data.clone(), 1);

        // Brand new authority set
        // Last authority set had set_id of 1
        // so while ingesting new authority set it
        // was incremented by 1.
        assert_authority_set(encoded_data.clone(), &second_authority_set, 1);
    }

    fn make_ids(keys: &[Ed25519Keyring]) -> AuthorityList {
        keys.iter()
            .map(|key| key.clone().public().into())
            .map(|id| (id, 1))
            .collect()
    }

    fn create_justification_commit(
        round: u64,
        set_id: u64,
        header_ancestry: Vec<Header>,
        peers: &[Keyring],
    ) -> Commit<Block> {
        assert!(header_ancestry.len() > 0);
        let first_header = header_ancestry.first().unwrap().clone();
        let mut precommits: Vec<SignedPrecommit<H256, u32, AuthoritySignature, AuthorityId>> =
            vec![];
        for header in header_ancestry {
            let precommit = Precommit::<Block> {
                target_hash: header.hash().clone(),
                target_number: *header.number(),
            };
            let msg = Message::<Block>::Precommit(precommit.clone());
            let mut encoded_msg: Vec<u8> = Vec::new();
            encoded_msg.clear();
            (&msg, round, set_id).encode_to(&mut encoded_msg);
            for peer in peers {
                let signature = peer.sign(&encoded_msg[..]).into();
                precommits.push(SignedPrecommit {
                    precommit: precommit.clone(),
                    signature,
                    id: peer.public().into(),
                });
            }
        }

        Commit::<Block> {
            target_hash: first_header.hash().clone(),
            target_number: *first_header.number(),
            precommits,
        }
    }

    #[test]
    fn test_finalization() {
        write_test_flow(format!("Starting Finalization test"));
        let peers = &[Ed25519Keyring::Alice];
        let voters = make_ids(peers);
        write_test_flow(format!("Creating initial authority set with one voter"));
        let genesis_authority_set = LightAuthoritySet::new(0, voters);
        write_test_flow(serde_json::to_string(&genesis_authority_set.authorities()).unwrap());

        write_test_flow(format!("\n\nInitializing database"));
        let (encoded_data, initial_header) =
            assert_successful_db_init(Some(genesis_authority_set.clone()), 1);
        let initial_block = Block::new(initial_header.clone(), vec![]);
        write_test_flow(serde_json::to_string(&initial_block).unwrap());
        let first_header = create_next_header(initial_header.clone());
        let encoded_data =
            assert_successful_header_ingestion(encoded_data, first_header.clone(), None, 1);

        // Now we will try to ingest a block with justification
        let second_header = create_next_header(first_header.clone());

        let third_header = create_next_header(second_header.clone());

        let fourth_header = create_next_header(third_header.clone());

        let fifth_header = create_next_header(fourth_header.clone());

        let sixth_header = create_next_header(fifth_header.clone());

        let header_ancestry = vec![
            second_header.clone(),
            third_header.clone(),
            fourth_header.clone(),
        ];

        let commit = create_justification_commit(1, 0, header_ancestry.clone(), peers);

        let grandpa_justification: GrandpaJustification<Block> = GrandpaJustification {
            round: 1,
            commit,
            votes_ancestries: header_ancestry[1..].to_vec(), // first_header.clone(), initial_header.clone()
        };

        let justification = Some(grandpa_justification.encode());

        write_test_flow(format!("\n\nCreated justification for Second header"));
        write_test_flow(format!(
            "Now we will try to ingest second header with justification"
        ));

        // Let's ingest it.
        let encoded_data = assert_successful_header_ingestion(
            encoded_data,
            second_header.clone(),
            justification,
            1,
        );

        // Finalized header should be updated
        assert_finalized_header(encoded_data.clone(), &second_header, 1);
        write_test_flow(format!("Initial, first and second header is finalized"));

        write_test_flow(format!("\n\nIngesting third header without justification"));
        let encoded_data =
            assert_successful_header_ingestion(encoded_data, third_header.clone(), None, 1);

        write_test_flow(format!("\n\nIngesting fourth header without justification"));
        let encoded_data =
            assert_successful_header_ingestion(encoded_data, fourth_header.clone(), None, 1);

        // Another justification, finalizing third, fourth and fifth header
        let header_ancestry = vec![fifth_header.clone(), sixth_header.clone()];

        let commit = create_justification_commit(1, 0, header_ancestry.clone(), peers);

        let grandpa_justification: GrandpaJustification<Block> = GrandpaJustification {
            round: 1,
            commit,
            votes_ancestries: header_ancestry[1..].to_vec(), // Sixth header
        };

        let justification = Some(grandpa_justification.encode());
        write_test_flow(format!("\n\nCreated justification for fifth header"));
        write_test_flow(format!(
            "Now we will try to ingest fifth header with justification"
        ));
        let encoded_data = assert_successful_header_ingestion(
            encoded_data,
            fifth_header.clone(),
            justification,
            1,
        );
        write_test_flow(format!("\n\n"));
        assert_finalized_header(encoded_data.clone(), &fifth_header, 1);
        write_test_flow(format!("third, fourth and fifth headers are now finalized"));
    }
}
