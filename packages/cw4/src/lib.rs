mod helpers;
mod msg;
mod query;

pub use crate::helpers::{Cw4CanonicalContract, Cw4Contract};
pub use crate::msg::{Cw4HandleMsg, Cw4InitMsg, Member};
pub use crate::query::{
    AdminResponse, Cw4QueryMsg, MemberListResponse, MemberResponse, TotalWeightResponse,
};

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
