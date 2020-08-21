# CW721 Spec: Non Fungible Tokens

CW721 is a specification for fungible tokens based on CosmWasm.
The name and design is based on Ethereum's ERC721 standard,
with some enhancements. The types in here can be imported by 
contracts that wish to implement this  spec, or by contracts that call 
to any standard cw721 contract.

The specification is split into multiple sections, a contract may only
implement some of this functionality, but must implement the base.

## Base

This handles ownership, transfers, and allowances. These must be supported
as is by all CW721 contracts. Note that all tokens must have an owner, 
as well as an ID. The ID is an arbitrary string, unique within the contract.

### Messages

`TransferNft{recipient, token_id}` - 
This transfers ownership of the token to `recipient` account. This is 
designed to send to an address controlled by a private key and *does not* 
trigger any actions on the recipient if it is a contract.

Requires `token_id` to point to a valid token, and `env.sender` to be 
the owner of it, or have an allowance to transfer it. 

`SendNft{contract, token_id, msg}` - 
This transfers ownership of the token to `contract` account. `contract` 
must be an address controlled by a smart contract, which implements
the CW721Receiver interface. The `msg` will be passed to the recipient 
contract, along with the token_id.

Requires `token_id` to point to a valid token, and `env.sender` to be 
the owner of it, or have an allowance to transfer it. 

`Approve{approved, token_id, expires}` - Grants permission to `approved` to
transfer or send the given token. This can only be performed when
`env.sender` is the owner of the given `token_id` or an `operator`. 
There can only be one approved account per token, and it is cleared once
the token  is transfered or sent.

`Revoke{token_id}` - This revokes a previously granted permission
to transfer the given `token_id`. This can only be granted when
`env.sender` is the owner of the given `token_id` or an `operator`.

`ApproveAll{operator, expires}` - Grant `operator` permission to transfer or send
all tokens owner by `env.sender`. This is tied to the owner, not the
tokens and applies to any future token that the owner receives as well.

`RevokeAll{operator}` - Revoke a previous `ApproveAll` permission granted
to the given `operator`.

### Queries

`OwnerOf{token_id}` - Returns the owner of the given token,
as well as anyone with approval on this particular token.
If the token is unknown, returns an error. Return type is
`OwnerResponse{owner}`.

`ApprovedForAll{owner}` - List all operators that can access all of 
the owner's tokens. Return type is `ApprovedForAllResponse`

`NumTokens{}` - Total number of tokens issued

### Receiver

The counter-part to `SendNft` is `ReceiveNft`, which must be implemented by
any contract that wishes to manage CW721 tokens. This is generally *not*
implemented by any CW721 contract.

`ReceiveNft{sender, token_id, msg}` - This is designed to handle `SendNft`
messages. The address of the contract is stored in `env.sender`
so it cannot be faked. The contract should ensure the sender matches
the token contract it expects to handle, and not allow arbitrary addresses.

The `sender` is the original account requesting to move the token
and `msg` is a `Binary` data that can be decoded into a contract-specific
message. This can be empty if we have only one default action,
or it may be a `ReceiveMsg` variant to clarify the intention. For example,
if I send to an exchange, I can specify the price I want to list the token 
for.
 
## Metadata

### Queries

`ContractInfo{}` - This returns top-level metadata about the contract.
Namely, `name` and `symbol`.

`NftInfo{token_id}` - This returns metadata about one particular token.
The return value is based on *ERC721 Metadata JSON Schema*, but directly
from the contract, not as a Uri. Only the image link is a Uri.

`AllNftInfo{token_id}` - This returns the result of both `NftInfo`
and `OwnerOf` as one query as an optimization for clients, which may
want both info to display one NFT.

## Enumerable

### Queries

**TODO** Handle paging? Right now, let it just get big and 
expensive to query. This API will most likely evolve.

`Tokens{owner}` - List all token_ids that belong to a given owner.
Return type is `TokensResponse{tokens: Vec<token_id>}`.

`AllTokens{}` - Requires pagination. Lists all token_ids controlled by 
the contract.
