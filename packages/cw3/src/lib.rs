// mod helpers;
mod msg;
mod query;

// pub use crate::helpers::{Cw3CanonicalContract, Cw3Contract};
pub use crate::msg::Cw3HandleMsg;
pub use crate::query::{Cw3QueryMsg, ProposalListResponse, ProposalResponse, VoteListResponse};

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
