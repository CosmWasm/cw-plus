mod helpers;
mod msg;
mod query;
mod receiver;

pub use crate::helpers::{Cw721CanonicalContract, Cw721Contract};
pub use crate::msg::{Cw721HandleMsg, Expiration};
pub use crate::query::{
    AllNftInfoResponse, ApprovedForAllResponse, ContractInfoResponse, Cw721QueryMsg,
    NftInfoResponse, NumTokensResponse, OwnerOfResponse, TokensResponse,
};
pub use crate::receiver::Cw721ReceiveMsg;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
