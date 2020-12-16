use cosmwasm_std::{Decimal as StdDecimal, Uint128};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use std::str::FromStr;

/// This defines the curves we are using.
///
/// I am struggling on what type to use for the math. Tokens are often stored as Uint128,
/// but they may have 6 or 9 digits. When using constant or linear functions, this doesn't matter
/// much, but for non-linear functions a lot more. Also, supply and reserve most likely have different
/// decimals... either we leave it for the callers to normalize and accept a `Decimal` input,
/// or we pass in `Uint128` as well as the decimal places for supply and reserve.
///
/// After working the first route and realizing that `Decimal` is not all that great to work with
/// when you want to do more complex math than add and multiply `Uint128`, I decided to go the second
/// route. That made the signatures quite complex and my final idea was to pass in `supply_decimal`
/// and `reserve_decimal` in the curve constructors.
pub trait Curve {
    /// Returns the spot price given the supply.
    /// `f(x)` from the README
    fn spot_price(&self, supply: Uint128) -> StdDecimal;

    /// Returns the total price paid up to purchase supply tokens (integral)
    /// `F(x)` from the README
    fn reserve(&self, supply: Uint128) -> Uint128;

    /// Inverse of reserve. Returns how many tokens would be issued
    /// with a total paid amount of reserve.
    /// `F^-1(x)` from the README
    fn supply(&self, reserve: Uint128) -> Uint128;
}

/// decimal returns an object = num * 10 ^ -scale
/// We use this function in contract.rs rather than call the crate constructor
/// itself, in case we want to swap out the implementation, we can do it only in this file.
pub fn decimal<T: Into<u128>>(num: T, scale: u32) -> Decimal {
    Decimal::from_i128_with_scale(num.into() as i128, scale)
}

/// StdDecimal stores as a u128 with 18 decimal points of precision
fn decimal_to_std(x: Decimal) -> StdDecimal {
    // this seems straight-forward (if inefficient), converting via string representation
    // TODO: handle errors better? Result?
    StdDecimal::from_str(&x.to_string()).unwrap()

    // // maybe a better approach doing math, not sure about rounding
    //
    // // try to preserve decimal points, max 9
    // let digits = min(x.scale(), 9);
    // let multiplier = 10u128.pow(digits);
    //
    // // we multiply up before we round off to u128,
    // // let StdDecimal do its best to keep these decimal places
    // let nominator = (x * decimal(multiplier, 0)).to_u128().unwrap();
    // StdDecimal::from_ratio(nominator, multiplier)
}

/// spot price is always a constant value
pub struct Constant {
    pub value: Decimal,
    pub normalize: DecimalPlaces,
}

impl Constant {
    pub fn new(value: Decimal, normalize: DecimalPlaces) -> Self {
        Self { value, normalize }
    }
}

impl Curve for Constant {
    // we need to normalize value with the reserve decimal places
    // (eg 0.1 value would return 100_000 if reserve was uatom)
    fn spot_price(&self, _supply: Uint128) -> StdDecimal {
        let out = self.value * self.normalize.to_reserve();
        decimal_to_std(out)
    }

    /// Returns total number of reserve tokens needed to purchase a given number of supply tokens.
    /// Note that both need to be normalized.
    fn reserve(&self, supply: Uint128) -> Uint128 {
        // supply * self.value
        let out = decimal(supply, 0) * self.value * self.normalize.supply_to_reserve();
        // TODO: handle overflow better? Result?
        out.floor().to_u128().unwrap().into()
    }

    fn supply(&self, reserve: Uint128) -> Uint128 {
        // reserve / self.value
        let out = decimal(reserve, 0) * self.normalize.reserve_to_supply() / self.value;
        // TODO: handle overflow better? Result?
        out.floor().to_u128().unwrap().into()
    }
}

/// spot_price is slope * supply
pub struct Linear {
    pub slope: Decimal,
    pub normalize: DecimalPlaces,
}

impl Curve for Linear {
    fn spot_price(&self, supply: Uint128) -> StdDecimal {
        // supply * self.value * (normalize.reserve / normalize.supply)
        let out = decimal(supply, 0) * self.slope * self.normalize.supply_to_reserve();
        decimal_to_std(out)
    }

    fn reserve(&self, supply: Uint128) -> Uint128 {
        // TODO: self.slope * supply * supply / 2
        let normalized = decimal(supply, self.normalize.supply);
        let square = normalized * normalized;
        // Note: multiplying by 0.5 is much faster than dividing by 2
        let out = square * self.slope * Decimal::new(5, 1);
        // TODO: handle overflow better? Result?
        out.floor().to_u128().unwrap().into()
    }

    fn supply(&self, _reserve: Uint128) -> Uint128 {
        // TODO: (2 * reserve / self.slope) ^ 0.5
        unimplemented!()
    }
}

/// DecimalPlaces should be passed into curve constructors
pub struct DecimalPlaces {
    /// Number of decimal places for the supply token (this is what was passed in cw20-base init
    pub supply: u32,
    /// Number of decimal places for the reserve token (eg. 6 for uatom, 9 for nstep, 18 for wei)
    pub reserve: u32,
}

impl DecimalPlaces {
    pub fn new(supply: u8, reserve: u8) -> Self {
        DecimalPlaces {
            supply: supply as u32,
            reserve: reserve as u32,
        }
    }

    pub fn to_reserve(&self) -> Decimal {
        Decimal::new(1, self.reserve)
    }

    pub fn from_reserve(&self) -> Decimal {
        Decimal::from_i128_with_scale(10i128.pow(self.reserve), 0)
    }

    pub fn to_supply(&self) -> Decimal {
        Decimal::new(1, self.supply)
    }

    pub fn from_supply(&self) -> Decimal {
        Decimal::from_i128_with_scale(10i128.pow(self.supply), 0)
    }

    pub fn supply_to_reserve(&self) -> Decimal {
        let mul = (self.reserve as i32) - (self.supply as i32);
        tens_exp(mul)
    }

    pub fn reserve_to_supply(&self) -> Decimal {
        let mul = (self.supply as i32) - (self.reserve as i32);
        tens_exp(mul)
    }
}

/// returns 10^exp
fn tens_exp(exp: i32) -> Decimal {
    let positive = exp > 0;
    let exp = exp.abs() as u32;
    if positive {
        // calculate the power
        Decimal::from_i128_with_scale(10i128.pow(exp), 0)
    } else {
        // 10 ^ -exp done automatically
        Decimal::new(1, exp)
    }
}

#[cfg(test)]
mod tests {
    // TODO: generic test that curve.supply(curve.reserve(supply)) == supply (or within some small rounding margin)

    // TODO: test DecimalPlaces return proper decimals

    // TODO: test Constant Curve behaves properly

    // TODO: test Linear Curve, what is implemented
}
