mod balance;
mod coin;
mod helpers;
mod msg;
mod query;
mod receiver;

pub use cw0::Expiration;

pub use crate::balance::Balance;
pub use crate::coin::{Cw20Coin, Cw20CoinHuman};
pub use crate::helpers::Cw20Contract;
pub use crate::msg::Cw20HandleMsg;
pub use crate::query::{
    AllAccountsResponse, AllAllowancesResponse, AllowanceInfo, AllowanceResponse, BalanceResponse,
    Cw20QueryMsg, MinterResponse, TokenInfoResponse,
};
pub use crate::receiver::Cw20ReceiveMsg;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
