use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{fmt, ops};

use cosmwasm_std::{Coin, OverflowError, OverflowOperation, StdError, StdResult, Uint128};

// Balance wraps Vec<Coin> and provides some nice helpers. It mutates the Vec and can be
// unwrapped when done.
#[derive(Serialize, Deserialize, Clone, Default, Debug, PartialEq, JsonSchema)]
pub struct NativeBalance(pub Vec<Coin>);

impl NativeBalance {
    pub fn into_vec(self) -> Vec<Coin> {
        self.0
    }

    /// returns true if the list of coins has at least the required amount
    pub fn has(&self, required: &Coin) -> bool {
        self.0
            .iter()
            .find(|c| c.denom == required.denom)
            .map(|m| m.amount >= required.amount)
            .unwrap_or(false)
    }

    /// normalize Wallet (sorted by denom, no 0 elements, no duplicate denoms)
    pub fn normalize(&mut self) {
        // drop 0's
        self.0.retain(|c| c.amount.u128() != 0);
        // sort
        self.0.sort_unstable_by(|a, b| a.denom.cmp(&b.denom));

        // find all i where (self[i-1].denom == self[i].denom).
        let mut dups: Vec<usize> = self
            .0
            .iter()
            .enumerate()
            .filter_map(|(i, c)| {
                if i != 0 && c.denom == self.0[i - 1].denom {
                    Some(i)
                } else {
                    None
                }
            })
            .collect();
        dups.reverse();

        // we go through the dups in reverse order (to avoid shifting indexes of other ones)
        for dup in dups {
            let add = self.0[dup].amount;
            self.0[dup - 1].amount += add;
            self.0.remove(dup);
        }
    }

    fn find(&self, denom: &str) -> Option<(usize, &Coin)> {
        self.0.iter().enumerate().find(|(_i, c)| c.denom == denom)
    }

    /// insert_pos should only be called when denom is not in the Wallet.
    /// it returns the position where denom should be inserted at (via splice).
    /// It returns None if this should be appended
    fn insert_pos(&self, denom: &str) -> Option<usize> {
        self.0.iter().position(|c| c.denom.as_str() >= denom)
    }

    pub fn is_empty(&self) -> bool {
        !self.0.iter().any(|x| x.amount != Uint128::zero())
    }

    /// similar to `Balance.sub`, but doesn't fail when minuend less than subtrahend
    pub fn sub_saturating(mut self, other: Coin) -> StdResult<Self> {
        match self.find(&other.denom) {
            Some((i, c)) => {
                if c.amount <= other.amount {
                    self.0.remove(i);
                } else {
                    self.0[i].amount = self.0[i].amount.checked_sub(other.amount)?;
                }
            }
            // error if no tokens
            None => {
                return Err(StdError::overflow(OverflowError::new(
                    OverflowOperation::Sub,
                    0,
                    other.amount.u128(),
                )))
            }
        };
        Ok(self)
    }
}

impl fmt::Display for NativeBalance {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for c in &self.0 {
            write!(f, "{}{}", c.denom, c.amount)?
        }
        Ok(())
    }
}

impl ops::AddAssign<Coin> for NativeBalance {
    fn add_assign(&mut self, other: Coin) {
        match self.find(&other.denom) {
            Some((i, c)) => {
                self.0[i].amount = c.amount + other.amount;
            }
            // place this in proper sorted order
            None => match self.insert_pos(&other.denom) {
                Some(idx) => self.0.insert(idx, other),
                None => self.0.push(other),
            },
        };
    }
}

impl ops::Add<Coin> for NativeBalance {
    type Output = Self;

    fn add(mut self, other: Coin) -> Self {
        self += other;
        self
    }
}

impl ops::AddAssign<NativeBalance> for NativeBalance {
    fn add_assign(&mut self, other: NativeBalance) {
        for coin in other.0.into_iter() {
            self.add_assign(coin);
        }
    }
}

impl ops::Add<NativeBalance> for NativeBalance {
    type Output = Self;

    fn add(mut self, other: NativeBalance) -> Self {
        self += other;
        self
    }
}

impl ops::Sub<Coin> for NativeBalance {
    type Output = StdResult<Self>;

    fn sub(mut self, other: Coin) -> StdResult<Self> {
        match self.find(&other.denom) {
            Some((i, c)) => {
                let remainder = c.amount.checked_sub(other.amount)?;
                if remainder.u128() == 0 {
                    self.0.remove(i);
                } else {
                    self.0[i].amount = remainder;
                }
            }
            // error if no tokens
            None => {
                return Err(StdError::overflow(OverflowError::new(
                    OverflowOperation::Sub,
                    0,
                    other.amount.u128(),
                )))
            }
        };
        Ok(self)
    }
}

impl ops::Sub<Vec<Coin>> for NativeBalance {
    type Output = StdResult<Self>;

    fn sub(self, amount: Vec<Coin>) -> StdResult<Self> {
        let mut res = self;
        for coin in amount {
            res = res.sub(coin.clone())?;
        }
        Ok(res)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use cosmwasm_std::coin;

    #[test]
    fn balance_has_works() {
        let balance = NativeBalance(vec![coin(555, "BTC"), coin(12345, "ETH")]);

        // less than same type
        assert!(balance.has(&coin(777, "ETH")));
        // equal to same type
        assert!(balance.has(&coin(555, "BTC")));

        // too high
        assert!(!balance.has(&coin(12346, "ETH")));
        // wrong type
        assert!(!balance.has(&coin(456, "ETC")));
    }

    #[test]
    fn balance_add_works() {
        let balance = NativeBalance(vec![coin(555, "BTC"), coin(12345, "ETH")]);

        // add an existing coin
        let more_eth = balance.clone() + coin(54321, "ETH");
        assert_eq!(
            more_eth,
            NativeBalance(vec![coin(555, "BTC"), coin(66666, "ETH")])
        );

        // add an new coin
        let add_atom = balance + coin(777, "ATOM");
        assert_eq!(
            add_atom,
            NativeBalance(vec![
                coin(777, "ATOM"),
                coin(555, "BTC"),
                coin(12345, "ETH"),
            ])
        );
    }

    #[test]
    fn balance_in_place_addition() {
        let mut balance = NativeBalance(vec![coin(555, "BTC")]);
        balance += coin(777, "ATOM");
        assert_eq!(
            &balance,
            &NativeBalance(vec![coin(777, "ATOM"), coin(555, "BTC")])
        );

        balance += NativeBalance(vec![coin(666, "ETH"), coin(123, "ATOM")]);
        assert_eq!(
            &balance,
            &NativeBalance(vec![coin(900, "ATOM"), coin(555, "BTC"), coin(666, "ETH")])
        );

        let sum = balance + NativeBalance(vec![coin(234, "BTC")]);
        assert_eq!(
            sum,
            NativeBalance(vec![coin(900, "ATOM"), coin(789, "BTC"), coin(666, "ETH")])
        );
    }

    #[test]
    fn balance_subtract_works() {
        let balance = NativeBalance(vec![coin(555, "BTC"), coin(12345, "ETH")]);

        // subtract less than we have
        let less_eth = (balance.clone() - coin(2345, "ETH")).unwrap();
        assert_eq!(
            less_eth,
            NativeBalance(vec![coin(555, "BTC"), coin(10000, "ETH")])
        );

        // subtract all of one coin (and remove with 0 amount)
        let no_btc = (balance.clone() - coin(555, "BTC")).unwrap();
        assert_eq!(no_btc, NativeBalance(vec![coin(12345, "ETH")]));

        // subtract more than we have
        let underflow = balance.clone() - coin(666, "BTC");
        assert!(underflow.is_err());

        // subtract non-existent denom
        let missing = balance - coin(1, "ATOM");
        assert!(missing.is_err());
    }

    #[test]
    fn balance_subtract_saturating_works() {
        let balance = NativeBalance(vec![coin(555, "BTC"), coin(12345, "ETH")]);

        // subtract less than we have
        let less_eth = balance.clone().sub_saturating(coin(2345, "ETH")).unwrap();
        assert_eq!(
            less_eth,
            NativeBalance(vec![coin(555, "BTC"), coin(10000, "ETH")])
        );

        // subtract all of one coin (and remove with 0 amount)
        let no_btc = balance.clone().sub_saturating(coin(555, "BTC")).unwrap();
        assert_eq!(no_btc, NativeBalance(vec![coin(12345, "ETH")]));

        // subtract more than we have
        let saturating = balance.clone().sub_saturating(coin(666, "BTC"));
        assert!(saturating.is_ok());
        assert_eq!(saturating.unwrap(), NativeBalance(vec![coin(12345, "ETH")]));

        // subtract non-existent denom
        let missing = balance - coin(1, "ATOM");
        assert!(missing.is_err());
    }

    #[test]
    fn normalize_balance() {
        // remove 0 value items and sort
        let mut balance = NativeBalance(vec![coin(123, "ETH"), coin(0, "BTC"), coin(8990, "ATOM")]);
        balance.normalize();
        assert_eq!(
            balance,
            NativeBalance(vec![coin(8990, "ATOM"), coin(123, "ETH")])
        );

        // merge duplicate entries of same denom
        let mut balance = NativeBalance(vec![
            coin(123, "ETH"),
            coin(789, "BTC"),
            coin(321, "ETH"),
            coin(11, "BTC"),
        ]);
        balance.normalize();
        assert_eq!(
            balance,
            NativeBalance(vec![coin(800, "BTC"), coin(444, "ETH")])
        );
    }
}
