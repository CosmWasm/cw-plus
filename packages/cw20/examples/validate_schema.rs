// use serde::{Deserialize, Serialize};
use cosmwasm_schema::schema_for;
use cosmwasm_std::{HumanAddr, Uint128};
use cw0::Expiration;
use cw20::{AllowanceInfo, AllowanceResponse};
use valico::json_schema;

// This is an example to show using serde_flatten works to produce the correct schema
fn main() {
    let info = AllowanceInfo {
        allowance_response: AllowanceResponse {
            allowance: Uint128(500000),
            expires: Expiration::AtTime(1234),
        },
        spender: HumanAddr::from("spender"),
    };

    let response = AllowanceResponse {
        allowance: Uint128(500000),
        expires: Expiration::AtTime(1234),
    };

    let allowance_info = serde_json::to_value(info).unwrap();
    let allowance_response = serde_json::to_value(response).unwrap();

    let schema = schema_for!(AllowanceInfo);
    let schema_string = serde_json::to_string(&schema).unwrap();
    let schema_json: serde_json::Value = serde_json::from_str(&schema_string).unwrap();

    let mut scope = json_schema::Scope::new();
    let r_schema = scope
        .compile_and_return(schema_json.clone(), true)
        .ok()
        .unwrap();

    println!("AllowanceInfo json schema \n {:#?}", schema_json);
    println!(
        "Shows valid schema w/ allowanceInfo: {}",
        r_schema.validate(&allowance_info).is_valid()
    );
    println!(
        "Shows invalid schema w/ allowanceResponse: {}",
        r_schema.validate(&allowance_response).is_valid()
    )
}
