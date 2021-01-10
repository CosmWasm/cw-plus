use cosmwasm_std::{MessageInfo, Uint128};
use thiserror::Error;

/// returns an error if any coins were sent
pub fn nonpayable(info: &MessageInfo) -> Result<(), PaymentError> {
    if info.sent_funds.is_empty() {
        Err(PaymentError::NonPayable {})
    } else {
        Ok(())
    }
}

/// Requires exactly one denom sent, which matches the requested denom.
/// Returns the amount if only one denom and non-zero amount. Errors otherwise.
pub fn must_pay(info: &MessageInfo, denom: &str) -> Result<Uint128, PaymentError> {
    let payment = match info.sent_funds.len() {
        0 => Err(PaymentError::NoFunds {}),
        1 => {
            if info.sent_funds[0].denom == denom {
                Ok(info.sent_funds[0].amount)
            } else {
                Err(PaymentError::MissingDenom(denom.to_string()))
            }
        }
        _ => Err(PaymentError::ExtraDenoms(denom.to_string())),
    }?;
    if payment.is_zero() {
        Err(PaymentError::NoFunds {})
    } else {
        Ok(payment)
    }
}

/// Similar to must_pay, but it any payment is optional. Returns an error if a different
/// denom was sent. Otherwise, returns the amount of `denom` sent, or 0 if nothing sent.
pub fn may_pay(info: &MessageInfo, denom: &str) -> Result<Uint128, PaymentError> {
    match info.sent_funds.len() {
        0 => Ok(Uint128(0)),
        1 => {
            if info.sent_funds[0].denom == denom {
                Ok(info.sent_funds[0].amount)
            } else {
                Err(PaymentError::ExtraDenoms(denom.to_string()))
            }
        }
        _ => Err(PaymentError::ExtraDenoms(denom.to_string())),
    }
}

#[derive(Error, Debug, PartialEq)]
pub enum PaymentError {
    #[error("Must send reserve token '{0}'")]
    MissingDenom(String),

    #[error("Sent unsupported token, must send reserve token '{0}'")]
    ExtraDenoms(String),

    #[error("No funds sent")]
    NoFunds {},

    #[error("This message does no accept funds")]
    NonPayable {},
}
