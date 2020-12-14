mod helpers;
mod hook;
mod msg;
mod query;

pub use crate::helpers::Cw4Contract;
pub use crate::hook::{MemberChangedHookMsg, MemberDiff};
pub use crate::msg::Cw4HandleMsg;
pub use crate::query::{
    member_key, AdminResponse, Cw4QueryMsg, HooksResponse, Member, MemberListResponse,
    MemberResponse, TotalWeightResponse, MEMBERS_CHANGELOG, MEMBERS_CHECKPOINTS, MEMBERS_KEY,
    TOTAL_KEY,
};

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
