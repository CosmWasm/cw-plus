use std::env::current_dir;
use std::fs::create_dir_all;

use cosmwasm_schema::{export_schema, export_schema_with_title, remove_schemas, schema_for};

use cw1_subkeys::msg::{AllAllowancesResponse, ExecuteMsg, QueryMsg};
use cw1_subkeys::state::Allowance;
use cw1_whitelist::msg::{AdminListResponse, InstantiateMsg};

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    export_schema(&schema_for!(InstantiateMsg), &out_dir);
    export_schema_with_title(&schema_for!(ExecuteMsg), &out_dir, "ExecuteMsg");
    export_schema_with_title(&schema_for!(QueryMsg), &out_dir, "QueryMsg");
    export_schema(&schema_for!(Allowance), &out_dir);
    export_schema(&schema_for!(AdminListResponse), &out_dir);
    export_schema(&schema_for!(AllAllowancesResponse), &out_dir);
}
