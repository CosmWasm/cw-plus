use cosmwasm_schema::write_api;

use cw1155::{Cw1155ExecuteMsg, Cw1155QueryMsg};
use cw1155_base::msg::InstantiateMsg;

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        execute: Cw1155ExecuteMsg,
        query: Cw1155QueryMsg,
    }
}
