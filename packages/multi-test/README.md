# Multi Test: Test helpers for multi-contract interactions

Warning: **Alpha Software** Designed for internal use only.

This is used for testing cw-plus contracts, we have no API
stability currently. We are working on refactoring it and will
expose a more refined version for use in other contracts. (Ideally
in cw-plus 0.9 or 0.10).

**Use at your own risk**

Let us run unit tests with contracts calling contracts, and calling
in and out of bank.

This only works with contracts and bank currently. We are working
on refactoring to make it more extensible for more handlers,
including custom messages/queries as well as IBC.


