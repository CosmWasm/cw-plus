use cosmwasm_schema::write_api;

pub use cw4::{AdminResponse, MemberListResponse, MemberResponse, TotalWeightResponse};
pub use cw4_group::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        execute: ExecuteMsg,
        query: QueryMsg,
    }
}
