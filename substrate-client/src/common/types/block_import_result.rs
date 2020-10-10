use crate::common::types::imported_aux::ImportedAux;

/// Block import successful result.
#[derive(Debug, PartialEq)]
pub enum BlockImportResult<N: ::std::fmt::Debug + PartialEq> {
    /// Imported known block.
    ImportedKnown(N),
    /// Imported unknown block.
    ImportedUnknown(N, ImportedAux),
}
