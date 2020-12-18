use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{BlockInfo, CanonicalAddr, StdResult, Storage, Uint128};
use cw0::Expiration;
use cw_storage_plus::Map;

// TODO: revisit design (split each claim on own key?)
pub const CLAIMS: Map<&[u8], Vec<Claim>> = Map::new("claim");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Claim {
    pub amount: Uint128,
    pub release_at: Expiration,
}

impl Claim {
    pub fn new(amount: u128, released: Expiration) -> Self {
        Claim {
            amount: amount.into(),
            release_at: released,
        }
    }
}

/// This creates a claim, such that the given address can claim an amount of tokens after
/// the release date.
pub fn create_claim(
    storage: &mut dyn Storage,
    addr: &CanonicalAddr,
    amount: Uint128,
    release_at: Expiration,
) -> StdResult<()> {
    // add a claim to this user to get their tokens after the unbonding period
    CLAIMS.update(storage, &addr, |old| -> StdResult<_> {
        let mut claims = old.unwrap_or_default();
        claims.push(Claim { amount, release_at });
        Ok(claims)
    })?;
    Ok(())
}

/// This iterates over all mature claims for the address, and removes them, up to an optional cap.
/// it removes the finished claims and returns the total amount of tokens to be released.
pub fn claim_tokens(
    storage: &mut dyn Storage,
    addr: &CanonicalAddr,
    block: &BlockInfo,
    cap: Option<Uint128>,
) -> StdResult<Uint128> {
    let mut to_send = Uint128(0);
    CLAIMS.update(storage, &addr, |claim| -> StdResult<_> {
        let (_send, waiting): (Vec<_>, _) =
            claim.unwrap_or_default().iter().cloned().partition(|c| {
                // if mature and we can pay fully, then include in _send
                if c.release_at.is_expired(block) {
                    if let Some(limit) = cap {
                        if to_send + c.amount > limit {
                            return false;
                        }
                    }
                    // TODO: handle partial paying claims?
                    to_send += c.amount;
                    true
                } else {
                    // not to send, leave in waiting and save again
                    false
                }
            });
        Ok(waiting)
    })?;
    Ok(to_send)
}
