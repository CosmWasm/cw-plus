use crate::{mock::*, ChannelOrder, ClientType, grandpa::consensus_state::ConsensusState, Error, Packet};
use frame_support::{assert_err, assert_ok, dispatch};
use sp_core::{Blake2Hasher, Hasher, H256};

fn create_client_func() {
	let identifier1 = Blake2Hasher::hash("appia".as_bytes());
	let identifier2 = Blake2Hasher::hash("flaminia".as_bytes());
	let height = 0;
	let consensus_state = ConsensusState {
		root: Blake2Hasher::hash("root".as_bytes()),
                height: 0,
		set_id: 0,
		authorities: vec![],
	};

	assert_ok!(IbcModule::create_client(identifier1, ClientType::GRANDPA, height.clone(), consensus_state.clone()));
	assert_ok!(IbcModule::create_client(identifier2, ClientType::GRANDPA, height.clone(), consensus_state.clone()));
	assert_err!(IbcModule::create_client(identifier1, ClientType::GRANDPA, height.clone(), consensus_state), Error::<Test>::ClientIdExist);
}

#[test]
fn create_client_should_work() {
	new_test_ext().execute_with(|| {
		create_client_func()
	});
}

fn bind_port_func() {
	let identifier = "bank".as_bytes().to_vec();
	let module_index = 45 as u8;
	assert_ok!(IbcModule::bind_port(identifier.clone(), module_index));
	assert_err!(IbcModule::bind_port(identifier.clone(), module_index), Error::<Test>::PortIdBinded);
}

#[test]
fn bind_port_should_work() {
	new_test_ext().execute_with(|| {
		bind_port_func();
	});
}

fn conn_open_init_func() {
    let identifier = Blake2Hasher::hash("appia-connection".as_bytes());
    let desired_counterparty_connection_identifier =
        Blake2Hasher::hash("flaminia-connection".as_bytes());
    let client_identifier =
        hex::decode("53a954d6a7b1c595e025226e5f2a1782fdea30cd8b0d207ed4cdb040af3bfa10").unwrap();
    let client_identifier = H256::from_slice(&client_identifier);
    let counterparty_client_identifier =
        hex::decode("779ca65108d1d515c3e4bc2e9f6d2f90e27b33b147864d1cd422d9f92ce08e03").unwrap();
    let counterparty_client_identifier = H256::from_slice(&counterparty_client_identifier);

    assert_err!(
        IbcModule::conn_open_init(
            identifier,
            desired_counterparty_connection_identifier,
            client_identifier,
            counterparty_client_identifier
        ),
        Error::<Test>::ClientIdNotExist
    );

    let identifier1 = Blake2Hasher::hash("appia".as_bytes());
    let height = 0;
    let consensus_state = ConsensusState {
        root: Blake2Hasher::hash("root".as_bytes()),
        height: 0,
        set_id: 0,
        authorities: vec![],
    };
    IbcModule::create_client(identifier1, ClientType::GRANDPA, height, consensus_state);

    assert_ok!(IbcModule::conn_open_init(
        identifier,
        desired_counterparty_connection_identifier,
        client_identifier,
        counterparty_client_identifier
    ));
    assert_err!(
        IbcModule::conn_open_init(
            identifier,
            desired_counterparty_connection_identifier,
            client_identifier,
            counterparty_client_identifier
        ),
        Error::<Test>::ConnectionIdExist
    );
}

#[test]
fn conn_open_init_should_work() {
    new_test_ext().execute_with(|| {
        conn_open_init_func();
    });
}

fn chan_open_init_func() {
    let module_index = 45 as u8;
    let order = ChannelOrder::Unordered;
    let connection_identifier =
        hex::decode("d93fc49e1b2087234a1e2fc204b500da5d16874e631e761bdab932b37907bd11").unwrap();
    let connection_identifier = H256::from_slice(&connection_identifier);
    let connection_hops = vec![connection_identifier];
    let port_identifier = "bank".as_bytes().to_vec();
    let channel_identifier = Blake2Hasher::hash(b"appia-channel");
    let counterparty_port_identifier = "bank".as_bytes().to_vec();
    let counterparty_channel_identifier = Blake2Hasher::hash(b"flaminia-channel");

    assert_err!(
        IbcModule::chan_open_init(
            module_index,
            order.clone(),
            connection_hops.clone(),
            port_identifier.clone(),
            channel_identifier,
            counterparty_port_identifier.clone(),
            counterparty_channel_identifier,
            vec![]
        ),
        Error::<Test>::ConnectionIdNotExist
    );

    let identifier1 = Blake2Hasher::hash("appia".as_bytes());
    let height = 0;
    let consensus_state = ConsensusState {
        root: Blake2Hasher::hash("root".as_bytes()),
        height: 0,
        set_id: 0,
        authorities: vec![],
    };
    IbcModule::create_client(identifier1, ClientType::GRANDPA, height, consensus_state);

    let identifier = Blake2Hasher::hash("appia-connection".as_bytes());
    let desired_counterparty_connection_identifier =
        Blake2Hasher::hash("flaminia-connection".as_bytes());
    let client_identifier =
        hex::decode("53a954d6a7b1c595e025226e5f2a1782fdea30cd8b0d207ed4cdb040af3bfa10").unwrap();
    let client_identifier = H256::from_slice(&client_identifier);
    let counterparty_client_identifier =
        hex::decode("779ca65108d1d515c3e4bc2e9f6d2f90e27b33b147864d1cd422d9f92ce08e03").unwrap();
    let counterparty_client_identifier = H256::from_slice(&counterparty_client_identifier);

    IbcModule::conn_open_init(
        identifier,
        desired_counterparty_connection_identifier,
        client_identifier,
        counterparty_client_identifier,
    );

    assert_err!(
        IbcModule::chan_open_init(
            module_index,
            order.clone(),
            connection_hops.clone(),
            port_identifier.clone(),
            channel_identifier,
            counterparty_port_identifier.clone(),
            counterparty_channel_identifier,
            vec![]
        ),
        Error::<Test>::PortIdNotMatch
    );

    IbcModule::bind_port("bank".as_bytes().to_vec(), module_index);

    assert_err!(
        IbcModule::chan_open_init(
            module_index,
            order.clone(),
            vec![],
            port_identifier.clone(),
            channel_identifier,
            counterparty_port_identifier.clone(),
            counterparty_channel_identifier,
            vec![]
        ),
        Error::<Test>::OnlyOneHopAllowedV1
    );

    assert_ok!(IbcModule::chan_open_init(
        module_index,
        order.clone(),
        connection_hops.clone(),
        port_identifier.clone(),
        channel_identifier,
        counterparty_port_identifier.clone(),
        counterparty_channel_identifier,
        vec![]
    ));

    assert_err!(
        IbcModule::chan_open_init(
            module_index,
            order.clone(),
            connection_hops.clone(),
            port_identifier.clone(),
            channel_identifier,
            counterparty_port_identifier.clone(),
            counterparty_channel_identifier,
            vec![]
        ),
        Error::<Test>::ChannelIdExist
    );
}

#[test]
fn chan_open_init_should_work() {
    new_test_ext().execute_with(|| {
        chan_open_init_func();
    });
}

fn send_packet_func() {
    let sequence = 1;
    let timeout_height = 1000;
    let source_port = "bank".as_bytes().to_vec();
    let source_channel =
        hex::decode("00e2e14470ed9a017f586dfe6b76bb0871a8c91c3151778de110db3dfcc286ac").unwrap();
    let source_channel = H256::from_slice(&source_channel);
    let dest_port = "bank".as_bytes().to_vec();
    let dest_channel =
        hex::decode("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa").unwrap();
    let dest_channel = H256::from_slice(&dest_channel);
    let data: Vec<u8> = hex::decode("01020304").unwrap();

    let mut packet = Packet {
        sequence,
        timeout_height,
        source_port,
        source_channel,
        dest_port,
        dest_channel,
        data,
    };

    assert_err!(
        IbcModule::send_packet(packet.clone()),
        Error::<Test>::PortIdNotMatch
    );

    let identifier1 = Blake2Hasher::hash("appia".as_bytes());
    let height = 0;
    let consensus_state = ConsensusState {
        root: Blake2Hasher::hash("root".as_bytes()),
        height: 0,
        set_id: 0,
        authorities: vec![],
    };
    IbcModule::create_client(identifier1, ClientType::GRANDPA, height, consensus_state);

    let identifier = Blake2Hasher::hash("appia-connection".as_bytes());
    let desired_counterparty_connection_identifier =
        Blake2Hasher::hash("flaminia-connection".as_bytes());
    let client_identifier =
        hex::decode("53a954d6a7b1c595e025226e5f2a1782fdea30cd8b0d207ed4cdb040af3bfa10").unwrap();
    let client_identifier = H256::from_slice(&client_identifier);
    let counterparty_client_identifier =
        hex::decode("779ca65108d1d515c3e4bc2e9f6d2f90e27b33b147864d1cd422d9f92ce08e03").unwrap();
    let counterparty_client_identifier = H256::from_slice(&counterparty_client_identifier);
    IbcModule::conn_open_init(
        identifier,
        desired_counterparty_connection_identifier,
        client_identifier,
        counterparty_client_identifier,
    );

    let module_index = 45 as u8;
    IbcModule::bind_port("bank".as_bytes().to_vec(), module_index);

    let order = ChannelOrder::Unordered;
    let connection_identifier =
        hex::decode("d93fc49e1b2087234a1e2fc204b500da5d16874e631e761bdab932b37907bd11").unwrap();
    let connection_identifier = H256::from_slice(&connection_identifier);
    let connection_hops = vec![connection_identifier];
    let port_identifier = "bank".as_bytes().to_vec();
    let channel_identifier = Blake2Hasher::hash(b"appia-channel");
    let counterparty_port_identifier = "bank".as_bytes().to_vec();
    let counterparty_channel_identifier = Blake2Hasher::hash(b"flaminia-channel");
    IbcModule::chan_open_init(
        module_index,
        order.clone(),
        connection_hops.clone(),
        port_identifier.clone(),
        channel_identifier,
        counterparty_port_identifier.clone(),
        counterparty_channel_identifier,
        vec![],
    );

    assert_err!(
        IbcModule::send_packet(packet.clone()),
        Error::<Test>::DestChannelIdNotMatch
    );

    let dest_channel =
        hex::decode("a1611bcd0ba368e921b1bd3eb4aa66534429b14837725e8cef28182c25db601e").unwrap();
    let dest_channel = H256::from_slice(&dest_channel);
    packet.dest_channel = dest_channel;
    assert_ok!(IbcModule::send_packet(packet.clone()));

    assert_err!(
        IbcModule::send_packet(packet.clone()),
        Error::<Test>::PackedSequenceNotMatch
    );
}

#[test]
fn send_packet_should_work() {
    new_test_ext().execute_with(|| {
        send_packet_func();
    });
}
