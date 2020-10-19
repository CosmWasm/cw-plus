# CW4 Spec: Group Members

CW4 is a spec for storing group membership, which can be combined
with CW3 multisigs. The purpose is to store a set of actors/voters
that can be accessed to determine permissions in another section.

Since this is often deployed as a contract pair, we expect this
contract to often be queried with `QueryRaw` and the internal
layout of some of the data structures becomes part of the public API.
Implementations may add more data structures, but at least
the ones laid out here should be under the specified keys and in the
same format.

In this case, a cw3 contract could *read* an external group contract with
no significant cost more than reading local storage. However, updating
that group contract (if allowed), would be an external message and
charged the instantiation overhead for each contract.

## Messages

We define an `InitMsg{admin, members}` to make it easy to set up a group
as part of another flow. Implementations should work with this setup,
but may add extra `Option<T>` fields for non-essential extensions to
configure in the `init` phase.

There are two messages supported by a group contract:

`UpdateAdmin{admin}` - changes (or clears) the admin for the contract

`UpdateMembers{add, remove}` - takes a membership diff and adds/updates the
  members, as well as removing any provided addresses. If an address is on both
  lists, it will be removed. If it appears multiple times in `add`, only the
  last occurance will be used.
  
Only the `admin` may execute either of these function. Thus, by omitting an
`admin`, we end up with a similar functionality ad `cw3-fixed-multisig`.
If we include one, it may often be desired to be a `cw3` contract that
uses this group contract as a group. This leads to a bit of chicken-and-egg
problem, but we will cover how to instantiate that in `cw3-flexible-multisig`
when the contract is built (TODO).

## Queries

### Smart

`Admin{}` - Returns the `admin` address, or `None` if unset.

`TotalWeight{}` - Returns the total weight of all current members,
  this is very useful if some conditions are defined on a "percentage of members".
  
`Member{addr}` - Returns the weight of this voter if they are a member of the
  group (may be 0), or `None` if they are not a member of the group.
  
 `MemberList{start_after, limit}` - Allows us to paginate over the list
   of all members. 0-weight members will be included. Removed members will not.

### Raw

In addition to the above "SmartQueries", which make up the public API,
we define two raw queries that are designed for more efficiency
in contract-contract calls. These use keys exported by `cw4`

`TOTAL_KEY` - making a raw query with this key (`b"total"`) will return a 
  JSON-encoded `u64`
  
 `members_key()` - takes a `CanonicalAddr` and returns a key that can be
   used for raw query (`"\x00\x07members" || addr`). This will return 
   empty bytes if the member is not inside the group, otherwise a 
   JSON-encoded `u64`