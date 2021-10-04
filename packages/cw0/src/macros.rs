#[macro_export]
macro_rules! ensure {
    ($cond:expr, $e:expr) => {
        if !($cond) {
            return Err($e);
        }
    };
}

#[macro_export]
macro_rules! ensure_generic {
    ($cond:expr, $e:expr) => {
        if !($cond) {
            return cosmwasm_std::StdError::generic_err($e);
        }
    };
}
