use crate::common::types::imported_aux::ImportedAux;

/// Block import result.
#[derive(Debug, PartialEq, Eq)]
pub enum ImportResult {
    /// Block imported.
    Imported(ImportedAux),
    /// Already in the blockchain.
    AlreadyInChain,
    /// Block or parent is known to be bad.
    KnownBad,
    /// Block parent is not in the chain.
    UnknownParent,
    /// Parent state is missing.
    MissingState,
}

impl ImportResult {
    /// Returns default value for `ImportResult::Imported` with
    /// `clear_justification_requests`, `needs_justification`,
    /// `bad_justification` and `needs_finality_proof` set to false.
    pub fn imported(is_new_best: bool) -> ImportResult {
        let mut aux = ImportedAux::default();
        aux.is_new_best = is_new_best;

        ImportResult::Imported(aux)
    }
}
