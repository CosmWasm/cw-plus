use schemars::schema::RootSchema;
use std::env::current_dir;
use std::fs::{create_dir_all, write};

use cosmwasm_schema::{remove_schemas, schema_for};

pub use cw4_group::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, QueryResponse};

// TODO: move this into export.rs (cosmwasm-schema)
#[derive(serde::Serialize)]
struct Api {
    instantiate: RootSchema,
    execute: RootSchema,
    query: RootSchema,
    response: RootSchema,
}

impl Api {
    pub fn set_names(&mut self) {
        if let Some(metadata) = &mut self.instantiate.schema.metadata {
            metadata.title = Some("InstantiateMsg".to_string());
        }
        if let Some(metadata) = &mut self.execute.schema.metadata {
            metadata.title = Some("ExecuteMsg".to_string());
        }
        if let Some(metadata) = &mut self.query.schema.metadata {
            metadata.title = Some("QueryMsg".to_string());
        }
        if let Some(metadata) = &mut self.response.schema.metadata {
            metadata.title = Some("QueryResponse".to_string());
        }
    }
}

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    let mut api = Api {
        instantiate: schema_for!(InstantiateMsg),
        execute: schema_for!(ExecuteMsg),
        query: schema_for!(QueryMsg),
        response: schema_for!(QueryResponse),
    };
    api.set_names();

    // TODO: expose write_schema in export.rs (cosmwasm-schema)
    let path = out_dir.join("api.json".to_string());
    let json = serde_json::to_string_pretty(&api).unwrap();
    write(&path, json + "\n").unwrap();
    println!("Created {}", path.to_str().unwrap());
}
