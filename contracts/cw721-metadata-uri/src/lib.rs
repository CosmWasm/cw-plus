use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub use cw721_base::{
    ContractError, ExecuteMsg, InstantiateMsg, MintMsg, MinterResponse, QueryMsg,
};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Extension {
    pub metadata_uri: String,
}

#[cfg(not(feature = "library"))]
pub mod entry {
    use super::*;

    pub use cw721_base::Cw721Contract;

    use cosmwasm_std::entry_point;
    use cosmwasm_std::{Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdResult};

    // This is a simple type to let us handle empty extensions

    // This makes a conscious choice on the various generics used by the contract
    #[entry_point]
    pub fn instantiate(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: InstantiateMsg,
    ) -> StdResult<Response> {
        let tract = Cw721Contract::<Extension, Empty>::default();
        tract.instantiate(deps, env, info, msg)
    }

    #[entry_point]
    pub fn execute(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: ExecuteMsg<Extension>,
    ) -> Result<Response, ContractError> {
        let tract = Cw721Contract::<Extension, Empty>::default();
        tract.execute(deps, env, info, msg)
    }

    #[entry_point]
    pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
        let tract = Cw721Contract::<Extension, Empty>::default();
        tract.query(deps, env, msg)
    }
}
