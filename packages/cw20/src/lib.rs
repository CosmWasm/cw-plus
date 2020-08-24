mod helpers;
mod msg;
mod query;
mod receiver;

pub use cw0::Expiration;

pub use crate::helpers::{Cw20CanonicalContract, Cw20Contract};
pub use crate::msg::Cw20HandleMsg;
pub use crate::query::{
    AllowanceResponse, BalanceResponse, Cw20QueryMsg, MinterResponse, TokenInfoResponse,
};
pub use crate::receiver::Cw20ReceiveMsg;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
