use cosmwasm_std::Empty;
use cosmwasm_schema::write_api;

use cw3_fixed_multisig::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        execute: ExecuteMsg<Empty>,
        query: QueryMsg,
    }
}
