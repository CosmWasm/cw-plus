use integer_cbrt::IntegerCubeRoot;
use integer_sqrt::IntegerSquareRoot;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

use cosmwasm_std::{Decimal as StdDecimal, Uint128};

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
    // TODO: execute errors better? Result?
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
        // f(x) = self.value
        decimal_to_std(self.value)
    }

    /// Returns total number of reserve tokens needed to purchase a given number of supply tokens.
    /// Note that both need to be normalized.
    fn reserve(&self, supply: Uint128) -> Uint128 {
        // f(x) = supply * self.value
        let reserve = self.normalize.from_supply(supply) * self.value;
        self.normalize.to_reserve(reserve)
    }

    fn supply(&self, reserve: Uint128) -> Uint128 {
        // f(x) = reserve / self.value
        let supply = self.normalize.from_reserve(reserve) / self.value;
        self.normalize.to_supply(supply)
    }
}

/// spot_price is slope * supply
pub struct Linear {
    pub slope: Decimal,
    pub normalize: DecimalPlaces,
}

impl Linear {
    pub fn new(slope: Decimal, normalize: DecimalPlaces) -> Self {
        Self { slope, normalize }
    }
}

impl Curve for Linear {
    fn spot_price(&self, supply: Uint128) -> StdDecimal {
        // f(x) = supply * self.value
        let out = self.normalize.from_supply(supply) * self.slope;
        decimal_to_std(out)
    }

    fn reserve(&self, supply: Uint128) -> Uint128 {
        // f(x) = self.slope * supply * supply / 2
        let normalized = self.normalize.from_supply(supply);
        let square = normalized * normalized;
        // Note: multiplying by 0.5 is much faster than dividing by 2
        let reserve = square * self.slope * Decimal::new(5, 1);
        self.normalize.to_reserve(reserve)
    }

    fn supply(&self, reserve: Uint128) -> Uint128 {
        // f(x) = (2 * reserve / self.slope) ^ 0.5
        // note: use addition here to optimize 2* operation
        let square = self.normalize.from_reserve(reserve + reserve) / self.slope;
        let supply = square_root(square);
        self.normalize.to_supply(supply)
    }
}

/// spot_price is slope * (supply)^0.5
pub struct SquareRoot {
    pub slope: Decimal,
    pub normalize: DecimalPlaces,
}

impl SquareRoot {
    pub fn new(slope: Decimal, normalize: DecimalPlaces) -> Self {
        Self { slope, normalize }
    }
}

impl Curve for SquareRoot {
    fn spot_price(&self, supply: Uint128) -> StdDecimal {
        // f(x) = self.slope * supply^0.5
        let square = self.normalize.from_supply(supply);
        let root = square_root(square);
        decimal_to_std(root * self.slope)
    }

    fn reserve(&self, supply: Uint128) -> Uint128 {
        // f(x) = self.slope * supply * supply^0.5 / 1.5
        let normalized = self.normalize.from_supply(supply);
        let root = square_root(normalized);
        let reserve = self.slope * normalized * root / Decimal::new(15, 1);
        self.normalize.to_reserve(reserve)
    }

    fn supply(&self, reserve: Uint128) -> Uint128 {
        // f(x) = (1.5 * reserve / self.slope) ^ (2/3)
        let base = self.normalize.from_reserve(reserve) * Decimal::new(15, 1) / self.slope;
        let squared = base * base;
        let supply = cube_root(squared);
        self.normalize.to_supply(supply)
    }
}

// we multiply by 10^18, turn to int, take square root, then divide by 10^9 as we convert back to decimal
fn square_root(square: Decimal) -> Decimal {
    // must be even
    // TODO: this can overflow easily at 18... what is a good value?
    const EXTRA_DIGITS: u32 = 12;
    let multiplier = 10u128.saturating_pow(EXTRA_DIGITS);

    // multiply by 10^18 and turn to u128
    let extended = square * decimal(multiplier, 0);
    let extended = extended.floor().to_u128().unwrap();

    // take square root, and build a decimal again
    let root = extended.integer_sqrt();
    decimal(root, EXTRA_DIGITS / 2)
}

// we multiply by 10^9, turn to int, take cube root, then divide by 10^3 as we convert back to decimal
fn cube_root(cube: Decimal) -> Decimal {
    // must be multiple of 3
    // TODO: what is a good value?
    const EXTRA_DIGITS: u32 = 9;
    let multiplier = 10u128.saturating_pow(EXTRA_DIGITS);

    // multiply out and turn to u128
    let extended = cube * decimal(multiplier, 0);
    let extended = extended.floor().to_u128().unwrap();

    // take cube root, and build a decimal again
    let root = extended.integer_cbrt();
    decimal(root, EXTRA_DIGITS / 3)
}

/// DecimalPlaces should be passed into curve constructors
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, JsonSchema, Default)]
pub struct DecimalPlaces {
    /// Number of decimal places for the supply token (this is what was passed in cw20-base instantiate
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

    pub fn to_reserve(self, reserve: Decimal) -> Uint128 {
        let factor = decimal(10u128.pow(self.reserve), 0);
        let out = reserve * factor;
        // TODO: execute overflow better? Result?
        out.floor().to_u128().unwrap().into()
    }

    pub fn to_supply(self, supply: Decimal) -> Uint128 {
        let factor = decimal(10u128.pow(self.supply), 0);
        let out = supply * factor;
        // TODO: execute overflow better? Result?
        out.floor().to_u128().unwrap().into()
    }

    pub fn from_supply(&self, supply: Uint128) -> Decimal {
        decimal(supply, self.supply)
    }

    pub fn from_reserve(&self, reserve: Uint128) -> Decimal {
        decimal(reserve, self.reserve)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // TODO: test DecimalPlaces return proper decimals

    #[test]
    fn constant_curve() {
        // supply is nstep (9), reserve is uatom (6)
        let normalize = DecimalPlaces::new(9, 6);
        let curve = Constant::new(decimal(15u128, 1), normalize);

        // do some sanity checks....
        // spot price is always 1.5 ATOM
        assert_eq!(
            StdDecimal::percent(150),
            curve.spot_price(Uint128::new(123))
        );

        // if we have 30 STEP, we should have 45 ATOM
        let reserve = curve.reserve(Uint128::new(30_000_000_000));
        assert_eq!(Uint128::new(45_000_000), reserve);

        // if we have 36 ATOM, we should have 24 STEP
        let supply = curve.supply(Uint128::new(36_000_000));
        assert_eq!(Uint128::new(24_000_000_000), supply);
    }

    #[test]
    fn linear_curve() {
        // supply is usdt (2), reserve is btc (8)
        let normalize = DecimalPlaces::new(2, 8);
        // slope is 0.1 (eg hits 1.0 after 10btc)
        let curve = Linear::new(decimal(1u128, 1), normalize);

        // do some sanity checks....
        // spot price is 0.1 with 1 USDT supply
        assert_eq!(
            StdDecimal::permille(100),
            curve.spot_price(Uint128::new(100))
        );
        // spot price is 1.7 with 17 USDT supply
        assert_eq!(
            StdDecimal::permille(1700),
            curve.spot_price(Uint128::new(1700))
        );
        // spot price is 0.212 with 2.12 USDT supply
        assert_eq!(
            StdDecimal::permille(212),
            curve.spot_price(Uint128::new(212))
        );

        // if we have 10 USDT, we should have 5 BTC
        let reserve = curve.reserve(Uint128::new(1000));
        assert_eq!(Uint128::new(500_000_000), reserve);
        // if we have 20 USDT, we should have 20 BTC
        let reserve = curve.reserve(Uint128::new(2000));
        assert_eq!(Uint128::new(2_000_000_000), reserve);

        // if we have 1.25 BTC, we should have 5 USDT
        let supply = curve.supply(Uint128::new(125_000_000));
        assert_eq!(Uint128::new(500), supply);
        // test square root rounding
        // TODO: test when supply has many more decimal places than reserve
        // if we have 1.11 BTC, we should have 4.7116875957... USDT
        let supply = curve.supply(Uint128::new(111_000_000));
        assert_eq!(Uint128::new(471), supply);
    }

    #[test]
    fn sqrt_curve() {
        // supply is utree (6) reserve is chf (2)
        let normalize = DecimalPlaces::new(6, 2);
        // slope is 0.35 (eg hits 0.35 after 1 chf, 3.5 after 100chf)
        let curve = SquareRoot::new(decimal(35u128, 2), normalize);

        // do some sanity checks....
        // spot price is 0.35 with 1 TREE supply
        assert_eq!(
            StdDecimal::percent(35),
            curve.spot_price(Uint128::new(1_000_000))
        );
        // spot price is 3.5 with 100 TREE supply
        assert_eq!(
            StdDecimal::percent(350),
            curve.spot_price(Uint128::new(100_000_000))
        );
        // spot price should be 23.478713763747788 with 4500 TREE supply (test rounding and reporting here)
        // rounds off around 8-9 sig figs (note diff for last points)
        assert_eq!(
            StdDecimal::from_ratio(2347871365u128, 100_000_000u128),
            curve.spot_price(Uint128::new(4_500_000_000))
        );

        // if we have 1 TREE, we should have 0.2333333333333 CHF
        let reserve = curve.reserve(Uint128::new(1_000_000));
        assert_eq!(Uint128::new(23), reserve);
        // if we have 100 TREE, we should have 233.333333333 CHF
        let reserve = curve.reserve(Uint128::new(100_000_000));
        assert_eq!(Uint128::new(23_333), reserve);
        // test rounding
        // if we have 235 TREE, we should have 840.5790828021146 CHF
        let reserve = curve.reserve(Uint128::new(235_000_000));
        assert_eq!(Uint128::new(84_057), reserve); // round down

        // // if we have 0.23 CHF, we should have 0.990453 TREE (round down)
        let supply = curve.supply(Uint128::new(23));
        assert_eq!(Uint128::new(990_000), supply);
        // if we have 840.58 CHF, we should have 235.000170 TREE (round down)
        let supply = curve.supply(Uint128::new(84058));
        assert_eq!(Uint128::new(235_000_000), supply);
    }

    // Idea: generic test that curve.supply(curve.reserve(supply)) == supply (or within some small rounding margin)
}
