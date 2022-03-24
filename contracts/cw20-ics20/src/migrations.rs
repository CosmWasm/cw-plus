// v1 format is anything older than 0.12.0
pub mod v1 {
    use schemars::JsonSchema;
    use serde::{Deserialize, Serialize};

    use cosmwasm_std::Addr;
    use cw_storage_plus::Item;

    #[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
    pub struct Config {
        pub default_timeout: u64,
        pub gov_contract: Addr,
    }

    pub const CONFIG: Item<Config> = Item::new("ics20_config");
}

// v2 format is anything older than 0.13.1 when we only updated the internal balances on success ack
pub mod v2 {
    use crate::state::{CHANNEL_INFO, CHANNEL_STATE};
    use crate::ContractError;
    use cosmwasm_std::{Coin, DepsMut, Env, Order, StdResult};

    pub fn update_balances(deps: DepsMut, env: &Env) -> Result<(), ContractError> {
        let channels = CHANNEL_INFO
            .keys(deps.storage, None, None, Order::Ascending)
            .collect::<StdResult<Vec<_>>>()?;
        match channels.len() {
            0 => Ok(()),
            1 => {
                let channel = &channels[0];
                let addr = &env.contract.address;
                let states = CHANNEL_STATE
                    .prefix(channel)
                    .range(deps.storage, None, None, Order::Ascending)
                    .collect::<StdResult<Vec<_>>>()?;
                for (denom, mut state) in states.into_iter() {
                    // this checks if we have received some coins that are "in flight" and not yet accounted in the state
                    let Coin { amount, .. } = deps.querier.query_balance(addr, &denom)?;
                    let diff = state.outstanding - amount;
                    // if they are in flight, we add them to the internal state now, as if we added them when sent (not when acked)
                    // to match the current logic
                    if !diff.is_zero() {
                        state.outstanding += diff;
                        state.total_sent += diff;
                        CHANNEL_STATE.save(deps.storage, (channel, &denom), &state)?;
                    }
                }
                Ok(())
            }
            _ => Err(ContractError::CannotMigrate {
                previous_contract: "multiple channels open".into(),
            }),
        }
    }
}
