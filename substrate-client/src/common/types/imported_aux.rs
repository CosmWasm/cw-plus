/// Auxiliary data associated with an imported block result.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct ImportedAux {
    /// Only the header has been imported. Block body verification was skipped.
    pub header_only: bool,
    /// Clear all pending justification requests.
    pub clear_justification_requests: bool,
    /// Request a justification for the given block.
    pub needs_justification: bool,
    /// Received a bad justification.
    pub bad_justification: bool,
    /// Request a finality proof for the given block.
    pub needs_finality_proof: bool,
    /// Whether the block that was imported is the new best block.
    pub is_new_best: bool,
}
