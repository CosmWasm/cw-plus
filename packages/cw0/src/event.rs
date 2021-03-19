use cosmwasm_std::Response;

/// This defines a set of attributes which should be added to `Response`.
pub trait Event {
    /// Append attributes to response
    fn add_attributes(&self, response: &mut Response);
}
