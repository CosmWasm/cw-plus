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

/// Opposite of ensure. If the condition (first argument) is true,
/// then return the second argument wrapped in Err(x).
///
///   fail_if!(!permissions.delegate, ContractError::DelegatePerm {});
///  is the same as
///   if !permissions.delegate {
///     return Err(ContractError::DelegatePerm {});
///   }
#[macro_export]
macro_rules! fail_if {
    ($cond:expr, $e:expr) => {
        if ($cond) {
            return Err($e);
        }
    };
}

/// Quick check for a guard. Like assert_eq!, but rather than panic,
/// it returns the second argument wrapped in Err(x).
///
///   ensure_eq!(info.sender, cfg.admin, ContractError::Unauthorized {});
///  is the same as
///   if info.sender != cfg.admin {
///     return Err(ContractError::Unauthorized {});
///   }
#[macro_export]
macro_rules! ensure_eq {
    ($a:expr, $b:expr, $e:expr) => {
        ensure!($a == $b, $e);
    };
}

#[cfg(test)]
mod test {
    use cosmwasm_std::StdError;

    #[test]
    fn ensure_works() {
        let check = |a, b| {
            ensure!(a == b, StdError::generic_err("foobar"));
            Ok(())
        };

        let err = check(5, 6).unwrap_err();
        assert!(matches!(err, StdError::GenericErr { .. }));

        check(5, 5).unwrap();
    }

    #[test]
    fn fail_if_works() {
        let check = |a, b| {
            fail_if!(a == b, StdError::generic_err("failure"));
            Ok(())
        };

        let err = check(5, 5).unwrap_err();
        assert!(matches!(err, StdError::GenericErr { .. }));

        check(5, 6).unwrap();
    }

    #[test]
    fn ensure_eq_works() {
        let check = |a, b| {
            ensure_eq!(a, b, StdError::generic_err("foobar"));
            Ok(())
        };

        let err = check("123", "456").unwrap_err();
        assert!(matches!(err, StdError::GenericErr { .. }));

        check("123", "123").unwrap();
    }
}
