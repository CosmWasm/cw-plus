use cosmwasm_std::{MessageInfo, Uint128};
use thiserror::Error;

/// returns an error if any coins were sent
pub fn nonpayable(info: &MessageInfo) -> Result<(), PaymentError> {
    if info.funds.is_empty() {
        Ok(())
    } else {
        Err(PaymentError::NonPayable {})
    }
}

/// Requires exactly one denom sent, which matches the requested denom.
/// Returns the amount if only one denom and non-zero amount. Errors otherwise.
pub fn must_pay(info: &MessageInfo, denom: &str) -> Result<Uint128, PaymentError> {
    match info.funds.len() {
        0 => Err(PaymentError::NoFunds {}),
        1 => {
            if info.funds[0].denom == denom {
                let payment = info.funds[0].amount;
                if payment.is_zero() {
                    Err(PaymentError::NoFunds {})
                } else {
                    Ok(payment)
                }
            } else {
                Err(PaymentError::MissingDenom(denom.to_string()))
            }
        }
        _ => {
            // find first mis-match
            let wrong = info.funds.iter().find(|c| c.denom != denom).unwrap();
            Err(PaymentError::ExtraDenom(wrong.denom.to_string()))
        }
    }
}

/// Similar to must_pay, but it any payment is optional. Returns an error if a different
/// denom was sent. Otherwise, returns the amount of `denom` sent, or 0 if nothing sent.
pub fn may_pay(info: &MessageInfo, denom: &str) -> Result<Uint128, PaymentError> {
    if info.funds.is_empty() {
        Ok(Uint128(0))
    } else if info.funds.len() == 1 && info.funds[0].denom == denom {
        Ok(info.funds[0].amount)
    } else {
        // find first mis-match
        let wrong = info.funds.iter().find(|c| c.denom != denom).unwrap();
        Err(PaymentError::ExtraDenom(wrong.denom.to_string()))
    }
}

#[derive(Error, Debug, PartialEq)]
pub enum PaymentError {
    #[error("Must send reserve token '{0}'")]
    MissingDenom(String),

    #[error("Received unsupported denom '{0}'")]
    ExtraDenom(String),

    #[error("No funds sent")]
    NoFunds {},

    #[error("This message does no accept funds")]
    NonPayable {},
}

#[cfg(test)]
mod test {
    use super::*;
    use cosmwasm_std::testing::mock_info;
    use cosmwasm_std::{coin, coins};

    const SENDER: &str = "sender";

    #[test]
    fn nonpayable_works() {
        let no_payment = mock_info(SENDER, &[]);
        nonpayable(&no_payment).unwrap();

        let payment = mock_info(SENDER, &coins(100, "uatom"));
        let res = nonpayable(&payment);
        assert_eq!(res.unwrap_err(), PaymentError::NonPayable {});
    }

    #[test]
    fn may_pay_works() {
        let atom: &str = "uatom";
        let no_payment = mock_info(SENDER, &[]);
        let atom_payment = mock_info(SENDER, &coins(100, atom));
        let eth_payment = mock_info(SENDER, &coins(100, "wei"));
        let mixed_payment = mock_info(SENDER, &[coin(50, atom), coin(120, "wei")]);

        let res = may_pay(&no_payment, atom).unwrap();
        assert_eq!(res, Uint128(0));

        let res = may_pay(&atom_payment, atom).unwrap();
        assert_eq!(res, Uint128(100));

        let err = may_pay(&eth_payment, atom).unwrap_err();
        assert_eq!(err, PaymentError::ExtraDenom("wei".to_string()));

        let err = may_pay(&mixed_payment, atom).unwrap_err();
        assert_eq!(err, PaymentError::ExtraDenom("wei".to_string()));
    }

    #[test]
    fn must_pay_works() {
        let atom: &str = "uatom";
        let no_payment = mock_info(SENDER, &[]);
        let atom_payment = mock_info(SENDER, &coins(100, atom));
        let zero_payment = mock_info(SENDER, &coins(0, atom));
        let eth_payment = mock_info(SENDER, &coins(100, "wei"));
        let mixed_payment = mock_info(SENDER, &[coin(50, atom), coin(120, "wei")]);

        let res = must_pay(&atom_payment, atom).unwrap();
        assert_eq!(res, Uint128(100));

        let err = must_pay(&no_payment, atom).unwrap_err();
        assert_eq!(err, PaymentError::NoFunds {});

        let err = must_pay(&zero_payment, atom).unwrap_err();
        assert_eq!(err, PaymentError::NoFunds {});

        let err = must_pay(&eth_payment, atom).unwrap_err();
        assert_eq!(err, PaymentError::MissingDenom(atom.to_string()));

        let err = must_pay(&mixed_payment, atom).unwrap_err();
        assert_eq!(err, PaymentError::ExtraDenom("wei".to_string()));
    }
}
