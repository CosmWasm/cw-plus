use cosmwasm_schema::write_api;
use cosmwasm_std::Empty;

use cw1_whitelist::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        execute: ExecuteMsg,
        query: QueryMsg<Empty>,
    }
}
