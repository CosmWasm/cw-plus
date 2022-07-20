use std::env::current_dir;
use std::fs::create_dir_all;

use cosmwasm_schema::{export_schema, remove_schemas, schema_for};

use cw20_ics20::msg::{
    ChannelResponse, ExecuteMsg, InitMsg, ListChannelsResponse, PortResponse, QueryMsg, TransferMsg,
};

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    export_schema(&schema_for!(InitMsg), &out_dir);
    export_schema(&schema_for!(ExecuteMsg), &out_dir);
    export_schema(&schema_for!(QueryMsg), &out_dir);
    export_schema(&schema_for!(TransferMsg), &out_dir);
    export_schema(&schema_for!(ChannelResponse), &out_dir);
    export_schema(&schema_for!(ListChannelsResponse), &out_dir);
    export_schema(&schema_for!(PortResponse), &out_dir);
}
