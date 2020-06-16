mod msg;
mod query;

pub use crate::msg::Cw20HandleMsg;
pub use crate::query::{AllowanceResponse, BalanceResponse, MetaResponse, MinterResponse, Cw20QueryMsg};


#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
