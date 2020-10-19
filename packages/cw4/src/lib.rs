mod helpers;
mod msg;
mod query;

pub use crate::helpers::{Cw4CanonicalContract, Cw4Contract};
pub use crate::msg::{Cw4HandleMsg, Cw4InitMsg, Member};
pub use crate::query::{
    member_key, AdminResponse, Cw4QueryMsg, MemberListResponse, MemberResponse,
    TotalWeightResponse, MEMBERS_KEY, TOTAL_KEY,
};

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
