use std::env::current_dir;
use std::fs::create_dir_all;

use cosmwasm_schema::{export_schema, remove_schemas, schema_for};

use cw721::{
    AllNftInfoResponse, ApprovedForAllResponse, ContractInfoResponse, Cw721HandleMsg,
    Cw721QueryMsg, Cw721ReceiveMsg, NftInfoResponse, NumTokensResponse, OwnerOfResponse,
    TokensResponse,
};

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    export_schema(&schema_for!(Cw721HandleMsg), &out_dir);
    export_schema(&schema_for!(Cw721QueryMsg), &out_dir);
    export_schema(&schema_for!(Cw721ReceiveMsg), &out_dir);
    export_schema(&schema_for!(AllNftInfoResponse), &out_dir);
    export_schema(&schema_for!(ApprovedForAllResponse), &out_dir);
    export_schema(&schema_for!(ContractInfoResponse), &out_dir);
    export_schema(&schema_for!(OwnerOfResponse), &out_dir);
    export_schema(&schema_for!(NftInfoResponse), &out_dir);
    export_schema(&schema_for!(NumTokensResponse), &out_dir);
    export_schema(&schema_for!(TokensResponse), &out_dir);
}
