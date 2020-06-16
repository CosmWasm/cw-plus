mod helpers;
mod msg;
mod query;

pub use crate::helpers::{ensure_cw20, ensure_cw20_allowance, ensure_cw20_mintable};
pub use crate::msg::Cw20HandleMsg;
pub use crate::query::{
    AllowanceResponse, BalanceResponse, Cw20QueryMsg, MetaResponse, MinterResponse,
};

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
