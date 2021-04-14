use std::env::current_dir;
use std::fs::create_dir_all;

use cosmwasm_schema::{export_schema, remove_schemas, schema_for};

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    export_schema(&schema_for!(cw1155::Cw1155ExecuteMsg), &out_dir);
    export_schema(&schema_for!(cw1155::Cw1155QueryMsg), &out_dir);
    export_schema(&schema_for!(cw1155::Cw1155ReceiveMsg), &out_dir);
    export_schema(&schema_for!(cw1155::Cw1155BatchReceiveMsg), &out_dir);
    export_schema(&schema_for!(cw1155::BalanceResponse), &out_dir);
    export_schema(&schema_for!(cw1155::BatchBalanceResponse), &out_dir);
    export_schema(&schema_for!(cw1155::ApprovedForAllResponse), &out_dir);
    export_schema(&schema_for!(cw1155::IsApprovedForAllResponse), &out_dir);
    export_schema(&schema_for!(cw1155::TokenInfoResponse), &out_dir);
    export_schema(&schema_for!(cw1155::TokensResponse), &out_dir);
}
