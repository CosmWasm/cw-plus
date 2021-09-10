use std::env::current_dir;
use std::fs::create_dir_all;

use cosmwasm_schema::{export_schema, remove_schemas, schema_for};

use cw2981::{CheckRoyaltiesResponse, Cw2981QueryMsg, RoyaltiesInfoResponse};
use cw2981::{ContractRoyaltiesInstantiateMsg, TokenRoyaltiesMintMsg};

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    export_schema(&schema_for!(ContractRoyaltiesInstantiateMsg), &out_dir);
    export_schema(&schema_for!(Cw2981QueryMsg), &out_dir);
    export_schema(&schema_for!(TokenRoyaltiesMintMsg), &out_dir);
    export_schema(&schema_for!(RoyaltiesInfoResponse), &out_dir);
    export_schema(&schema_for!(CheckRoyaltiesResponse), &out_dir);
}
