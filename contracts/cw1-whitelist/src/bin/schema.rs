use cosmwasm_std::Empty;
use cosmwasm_schema::write_api;

use cw1_whitelist::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        execute: ExecuteMsg<Empty>,
        query: QueryMsg<Empty>,
    }
}
