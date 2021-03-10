# CW1155 Spec: Multiple Tokens

CW1155 is a specification for managing multiple tokens based on CosmWasm.
The name and design is based on Ethereum's ERC1155 standard.

The specification is split into multiple sections, a contract may only
implement some of this functionality, but must implement the base.

## Base

### Messages

`TransferFrom{from, to, token_id, value}` - This transfers some amount of tokens between two accounts. The operator should have approval from the source account.

Both `from` and `to` are `Option`, if `from` is `None`, it means mint new tokens, if `to` is `None`, it means burn tokens. They must not both be `None` at the same time.

`SendFrom{from, to, token_id, value, msg}` - This transfers some amount of tokens between two accounts. `to` 
must be an address controlled by a smart contract, which implements
the `CW1155Receiver` interface. The operator should have approval from the source account. The `msg` will be passed to the recipient contract, along with the other fields.

`from` is `Option`, `None` means minting new tokens.

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

## Metadata and Enumerable

[TODO] ERC1155 suggests that metadata and enumerable should be indexed from events log, to save some on-chain storage. Should we define standard events like ERC1155?