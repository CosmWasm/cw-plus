use std::env::current_dir;
use std::fs::create_dir_all;

use cosmwasm_schema::{export_schema, export_schema_with_title, remove_schemas, schema_for};

use cw20_atomic_swap::msg::HandleMsg;
use cw20_atomic_swap::msg::InitMsg;
use cw20_atomic_swap::msg::QueryMsg;
use cw20_atomic_swap::msg::ListResponse;
use cw20_atomic_swap::msg::DetailsResponse;

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    export_schema(&schema_for!(InitMsg), &out_dir);
    export_schema_with_title(&mut schema_for!(HandleMsg), &out_dir, "HandleMsg");
    export_schema(&schema_for!(QueryMsg), &out_dir);
    export_schema(&schema_for!(ListResponse), &out_dir);
    export_schema(&schema_for!(DetailsResponse), &out_dir);
}
