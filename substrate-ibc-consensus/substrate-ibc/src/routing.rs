use crate::{ChannelOrder, Packet};
use sp_core::H256;
use sp_std::prelude::*;

pub trait ModuleCallbacks {
    fn on_chan_open_try(
        index: usize,
        order: ChannelOrder,
        connection_hops: Vec<H256>,
        port_identifier: Vec<u8>,
        channel_identifier: H256,
        counterparty_port_identifier: Vec<u8>,
        counterparty_channel_identifier: H256,
        version: Vec<u8>,
        counterparty_version: Vec<u8>,
    );
    fn on_chan_open_ack(
        index: usize,
        port_identifier: Vec<u8>,
        channel_identifier: H256,
        version: Vec<u8>,
    );
    fn on_chan_open_confirm(index: usize, port_identifier: Vec<u8>, channel_identifier: H256);
    fn on_recv_packet(index: usize, packet: Packet);
}

// fn conn_open_try() {}
// fn conn_open_ack() {}
// fn conn_open_confirm() {}
//
// fn chan_open_try() {}
// fn chan_open_ack() {}
// fn chan_open_confirm() {}
// fn chan_close_confirm() {}
//
// fn send_packet() {}
// fn recv_packet() {}
// fn acknowledge_packet() {}
// fn timeout_packet() {}
// fn timeout_on_close() {}
// fn cleanup_packet() {}
