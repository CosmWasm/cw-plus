use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// To allow for royalties to be specified AT TOKEN LEVEL
/// (i.e. different NFTs minted by this contract might
/// have different royalty info),
/// we make some changes to the base MintMsg
/// This, or custom logic like it, should be used
/// in contracts that implement 2981 royalties
/// as defined by the query interface
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TokenRoyaltiesMintMsg {
    /// Unique ID of the NFT
    pub token_id: String,
    /// The owner of the newly minter NFT
    pub owner: String,
    /// Identifies the asset to which this NFT represents
    pub name: String,
    /// Describes the asset to which this NFT represents (may be empty)
    pub description: Option<String>,
    /// A URI pointing to an image representing the asset
    pub image: Option<String>,

    // params related to royalties
    /// Whether or not royalties should be paid
    pub royalty_payments: bool,
    /// The percentage of sale that is owed
    /// note that in future this could be extended
    /// to have an additional parameter for e.g. decay
    /// so a royalty could decrease over time
    /// this is merely the base case
    pub royalty_percentage: Option<u32>,
    /// The address that should be paid the royalty
    pub royalty_payment_address: Option<String>,
}

/// To allow for royalties to be specified AT CONTRACT LEVEL
/// (i.e. for all tokens managed by this contract),
/// you would specify royalties at instantiation time
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ContractRoyaltiesInstantiateMsg {
    /// Name of the NFT contract
    pub name: String,
    /// Symbol of the NFT contract
    pub symbol: String,

    /// The minter is the only one who can create new NFTs.
    /// This is designed for a base NFT that is controlled by an external program
    /// or contract. You will likely replace this with custom logic in custom NFTs
    pub minter: String,

    /// Does this implement CW-2981
    pub royalty_payments: bool,
    /// What is the royalty percentage to use?
    pub royalty_percentage: Option<u32>,
    /// Where should royalty payments be sent?
    pub royalty_payment_address: Option<String>,
}
