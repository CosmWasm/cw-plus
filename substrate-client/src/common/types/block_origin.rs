/// Block data origin.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum BlockOrigin {
    /// Genesis block built into the client.
    Genesis,
    /// Block is part of the initial sync with the network.
    NetworkInitialSync,
    /// Block was broadcasted on the network.
    NetworkBroadcast,
    /// Block that was received from the network and validated in the consensus process.
    ConsensusBroadcast,
    /// Block that was collated by this node.
    Own,
    /// Block was imported from a file.
    File,
}
