/*!
CW1155 is a specification for managing multiple tokens based on CosmWasm.
The name and design is based on Ethereum's ERC1155 standard.

The specification is split into multiple sections, a contract may only
implement some of this functionality, but must implement the base.

Fungible tokens and non-fungible tokens are treated equally, non-fungible tokens just have one max supply.

Approval is set or unset to some operator over entire set of tokens. (More nuanced control is defined in
[ERC1761](https://eips.ethereum.org/EIPS/eip-1761))

For more information on this specification, please check out the
[README](https://github.com/CosmWasm/cw-plus/blob/main/packages/cw1155/README.md).
*/

pub use cw_utils::Expiration;

pub use crate::event::{ApproveAllEvent, MetadataEvent, TransferEvent};
pub use crate::msg::{Cw1155ExecuteMsg, TokenId};
pub use crate::query::{
    Approval, ApprovedForAllResponse, BalanceResponse, BatchBalanceResponse, Cw1155QueryMsg,
    IsApprovedForAllResponse, TokenInfoResponse, TokensResponse,
};
pub use crate::receiver::{Cw1155BatchReceiveMsg, Cw1155ReceiveMsg};

mod event;
mod msg;
mod query;
mod receiver;
