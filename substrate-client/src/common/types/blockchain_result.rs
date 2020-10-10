use crate::common::types::blockchain_error::BlockchainError;

pub type BlockchainResult<T> = Result<T, BlockchainError>;
