use cosmwasm_std::Decimal;

/// This defines the curves we are using.
///
/// I am struggling on what type to use for the math. Tokens are often stored as Uint128,
/// but they may have 6 or 9 digits. When using constant or linear functions, this doesn't matter
/// much, but for non-linear functions a lot more. Also, supply and reserve most likely have different
/// decimals... let's leave it for the callers to normalize this.
///
/// Decimal is a relatively simple type integrated with Uint128. It supports up to 18 digits behind
/// the decimal point and 20 in front, which should capture any token value we can reasonably imagine.
/// This can make it a nice type for the interface.
///
/// Internally, if we want to do exponents, etc, we will likely want another type that can handle
/// that better - but that would be determined by each implementation.
pub trait Curve {
    /// Returns the spot price given the supply.
    /// `f(x)` from the README
    fn spot_price(&self, supply: Decimal) -> Decimal;

    /// Returns the total price paid up to purchase supply tokens (integral)
    /// `F(x)` from the README
    fn reserve(&self, supply: Decimal) -> Decimal;

    /// Inverse of reserve. Returns how many tokens would be issued
    /// with a total paid amount of reserve.
    /// `F^-1(x)` from the README
    fn supply(&self, reserve: Decimal) -> Decimal;
}

/// spot price is always this value
pub struct Constant(pub Decimal);

impl Curve for Constant {
    fn spot_price(&self, _supply: Decimal) -> Decimal {
        self.0
    }

    fn reserve(&self, _supply: Decimal) -> Decimal {
        // TODO: supply * self.0
        unimplemented!()
    }

    fn supply(&self, _reserve: Decimal) -> Decimal {
        // TODO: reserve / self.0
        unimplemented!()
    }
}

// TODO: generic test that curve.supply(curve.reserve(supply)) == supply (or within some small rounding margin)

/// spot_price is slope * supply
pub struct Linear {
    pub slope: Decimal,
}

impl Curve for Linear {
    fn spot_price(&self, _supply: Decimal) -> Decimal {
        // TODO: self.slope * supply
        unimplemented!()
    }

    fn reserve(&self, _supply: Decimal) -> Decimal {
        // TODO: self.slope * supply * supply / 2
        unimplemented!()
    }

    fn supply(&self, _reserve: Decimal) -> Decimal {
        // TODO: (2 * reserve / self.slope) ^ 0.5
        unimplemented!()
    }
}
