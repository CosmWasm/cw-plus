use std::env::current_dir;
use std::fs::create_dir_all;

use cosmwasm_schema::{export_schema, export_schema_with_title, remove_schemas, schema_for};

use cw1_whitelist_ng::msg::*;

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    export_schema(&schema_for!(InstantiateMsg), &out_dir);
    export_schema_with_title(&schema_for!(Cw1ExecMsg), &out_dir, "Cw1ExecMsg");
    export_schema_with_title(&schema_for!(WhitelistExecMsg), &out_dir, "WhitelistExecMsg");
    export_schema_with_title(&schema_for!(Cw1QueryMsg), &out_dir, "Cw1QueryMsg");
    export_schema_with_title(
        &schema_for!(WhitelistQueryMsg),
        &out_dir,
        "WhitelistQueryMsg",
    );
    export_schema(&schema_for!(AdminListResponse), &out_dir);
}
