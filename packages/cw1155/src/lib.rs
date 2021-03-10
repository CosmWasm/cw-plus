pub use crate::msg::{Cw1155HandleMsg, TokenId};
pub use crate::query::{
    ApprovedForAllResponse, BalanceResponse, BatchBalanceResponse, Cw1155QueryMsg,
};
pub use crate::receiver::{Cw1155BatchReceiveMsg, Cw1155ReceiveMsg};

mod msg;
mod query;
mod receiver;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
