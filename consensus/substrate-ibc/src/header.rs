use crate::client::ClientType;

pub trait Header {
    /// The type of client (eg. Tendermint)
    fn client_type(&self) -> ClientType;

    /// The height of the consensus state
    fn height(&self) -> u32;
}
