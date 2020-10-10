use crate::common::types::blockchain_result::BlockchainResult;

/// Provides access to an auxiliary database.
pub trait AuxStore {
    /// Insert auxiliary data into key-value store.
    ///
    /// Deletions occur after insertions.
    fn insert_aux<
        'a,
        'b: 'a,
        'c: 'a,
        I: IntoIterator<Item = &'a (&'c [u8], &'c [u8])>,
        D: IntoIterator<Item = &'a &'b [u8]>,
    >(
        &self,
        insert: I,
        delete: D,
    ) -> BlockchainResult<()>;

    /// Query auxiliary data from key-value store.
    fn get_aux(&self, key: &[u8]) -> BlockchainResult<Option<Vec<u8>>>;
}
