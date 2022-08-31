use cosmwasm_schema::write_api;

pub use cw4::{AdminResponse, MemberListResponse, MemberResponse, TotalWeightResponse};
pub use cw4_stake::msg::{
    ClaimsResponse, ExecuteMsg, InstantiateMsg, QueryMsg, ReceiveMsg, StakedResponse,
};

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        execute: ExecuteMsg,
        query: QueryMsg,
    }
}
