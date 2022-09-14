use cosmwasm_schema::write_api;

use cw1_subkeys::msg::{ExecuteMsg, QueryMsg};

use cw1_whitelist::msg::InstantiateMsg;

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        execute: ExecuteMsg,
        query: QueryMsg,
    }
}
