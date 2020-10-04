use crate::{ChannelOrder, Module, ModuleCallbacks, Packet, Trait};
use sp_core::H256;
use frame_support::{impl_outer_origin, parameter_types, weights::Weight};
use sp_runtime::{
	traits::{BlakeTwo256, IdentityLookup}, testing::Header, Perbill,
};
use frame_system as system;

impl_outer_origin! {
	pub enum Origin for Test {}
}

// Configure a mock runtime to test the pallet.

#[derive(Clone, Eq, PartialEq)]
pub struct Test;
parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const MaximumBlockWeight: Weight = 1024;
	pub const MaximumBlockLength: u32 = 2 * 1024;
	pub const AvailableBlockRatio: Perbill = Perbill::from_percent(75);
}

impl system::Trait for Test {
	type BaseCallFilter = ();
	type Origin = Origin;
	type Call = ();
	type Index = u64;
	type BlockNumber = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = u64;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = ();
	type BlockHashCount = BlockHashCount;
	type MaximumBlockWeight = MaximumBlockWeight;
	type DbWeight = ();
	type BlockExecutionWeight = ();
	type ExtrinsicBaseWeight = ();
	type MaximumExtrinsicWeight = MaximumBlockWeight;
	type MaximumBlockLength = MaximumBlockLength;
	type AvailableBlockRatio = AvailableBlockRatio;
	type Version = ();
	type PalletInfo = ();
	type AccountData = ();
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
}

pub struct ModuleCallbacksImpl;

impl ModuleCallbacks for ModuleCallbacksImpl {
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
	) {
		unimplemented!()
	}

	fn on_chan_open_ack(
        index: usize,
        port_identifier: Vec<u8>,
        channel_identifier: H256,
        version: Vec<u8>,
	) {
		unimplemented!()
	}

	fn on_chan_open_confirm(index: usize, port_identifier: Vec<u8>, channel_identifier: H256) {
		unimplemented!()
	}

	fn on_recv_packet(index: usize, packet: Packet) {
		unimplemented!()
	}
}

impl Trait for Test {
	type Event = ();
	type ModuleCallbacks = ModuleCallbacksImpl;
}

pub type IbcModule = Module<Test>;

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	system::GenesisConfig::default().build_storage::<Test>().unwrap().into()
}
