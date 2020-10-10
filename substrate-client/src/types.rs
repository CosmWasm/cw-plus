use sp_runtime::{traits::BlakeTwo256, OpaqueExtrinsic};

pub type BlockNumber = u32;

pub type Header = sp_runtime::generic::Header<BlockNumber, BlakeTwo256>;

pub type Block = sp_runtime::generic::Block<Header, OpaqueExtrinsic>;

pub type SignedBlock = sp_runtime::generic::SignedBlock<Block>;
