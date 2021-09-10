mod msg;
mod query;

pub use crate::msg::{ContractRoyaltiesInstantiateMsg, TokenRoyaltiesMintMsg};
pub use crate::query::{CheckRoyaltiesResponse, Cw2981QueryMsg, RoyaltiesInfoResponse};

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        // test me
    }
}
