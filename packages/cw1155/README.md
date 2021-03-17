# CW1155 Spec: Multiple Tokens

CW1155 is a specification for managing multiple tokens based on CosmWasm.
The name and design is based on Ethereum's ERC1155 standard.

Design decisions:

- Fungible tokens and non-fungible tokens are treated equally, non-fungible tokens just have one max supply.

- Approval is set or unset to some operator over entire set of tokens. (More nuanced control is defined in [ERC1761](https://eips.ethereum.org/EIPS/eip-1761), do we want to merge them together?)

- Metadata and token enumeration should be done by indexing events off-chain.

- Mint and burn are mixed with transfer/send messages, otherwise, we'll have much more message types, e.g. `Mint`/`MintToContract`/`BatchMint`/`BatchMintToContract`, etc.

  In transfer/send messges, `from`/`to` are optional, a `None` `from` means minting, a `None` `to` means burning, they must not both be `None` at the same time.

## Base

### Messages

`TransferFrom{from, to, token_id, value}` - This transfers some amount of tokens between two accounts. The operator should have approval from the source account.

`SendFrom{from, to, token_id, value, msg}` - This transfers some amount of tokens between two accounts. `to` 
must be an address controlled by a smart contract, which implements
the `CW1155Receiver` interface. The operator should have approval from the source account. The `msg` will be passed to the recipient contract, along with the other fields.

`BatchTransferFrom{from, to, batch}` - Batched version of `TransferFrom` which can handle multiple types of tokens at once.

`BatchSendFrom{from, contract, batch, msg}` - Batched version of `SendFrom` which can handle multiple types of tokens at once.

`SetApprovalForAll { operator, approved }` - Grant or revoke  `operator` the permission to transfer or send all tokens owned by `msg.sender`. This approval is tied to the owner, not the
tokens and applies to any future token that the owner receives as well.

### Queries

`Balance { owner, token_id }` - Query the balance of `owner` on perticular type of token, default to `0` when record not exist.

`BatchBalance { owner, token_ids }` - Query the balance of `owner` on multiple types of tokens, batched version of `Balance`

`ApprovedForAll{ owner, spender }` - Query if `spender` has the permission to transfer or send tokens owned by `msg.sender`.

### Receiver

Any contract wish to receive CW1155 tokens must implement `Cw1155ReceiveMsg` and `Cw1155BatchReceiveMsg`.

`Cw1155ReceiveMsg { operator, from, token_id, amount, msg}` - 

`Cw1155BatchReceiveMsg { operator, from, batch, msg}` - 

### Events

- `transfer(from, to, token_id, value)`

  `from`/`to` are optional, no `from` attribute means minting, no `to` attribute means burning.

- `metadata(url, token_id)`

  Metadata url of `token_id` is changed, `url` should point to a json file.

## Metadata and Enumerable

[TODO] ERC1155 suggests that metadata and enumerable should be indexed from events log, to save some on-chain storage. Should we define standard events like ERC1155?