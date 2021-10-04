/// Quick check for a guard. If the condition (first argument) is false,
/// then return the second argument wrapped in Err(x).
///
///   ensure!(permissions.delegate, ContractError::DelegatePerm {});
///  is the same as
///   if !permissions.delegate {
///     return Err(ContractError::DelegatePerm {});
///   }
#[macro_export]
macro_rules! ensure {
    ($cond:expr, $e:expr) => {
        if !($cond) {
            return Err($e);
        }
    };
}

/// Returns a generic error. In general, use ensure! with a specific ContractError variant,
/// but some places we don't have one. This can make quick error messages in such cases.
/// Uses .into() so that it can return StdError or any Error type with From<StdError> implemented.
///
///   ensure_generic!(id > 0, "Bad ID");
/// is the same as
///   if !(id > 0) {
///     return Err(StdError::generic_err("Bad ID").into);
///   }
#[macro_export]
macro_rules! ensure_generic {
    ($cond:expr, $e:expr) => {
        if !($cond) {
            return Err(cosmwasm_std::StdError::generic_err($e).into());
        }
    };
}
