# Security Policy

## Supported Versions

cw-plus is still pre v1.0. A best effort has been made that the contracts here are secure, and we have moved the more
experimental contracts into community repositories like [cw-nfts](https://github.com/CosmWasm/cw-nfts) and
[cw-tokens](https://github.com/CosmWasm/cw-tokens). That said, we have not done an audit on them (formal or informal)
and you can use them at your own risk. We highly suggest doing your own audit on any contract you plan to deploy
with significant token value, and please inform us if it detects any issues so we can upstream them.

Until v1.0 APIs are subject to change. The contracts APIs are pretty much stable, most work is currently
in `storage-plus` and `multi-test`.

## Reporting a Vulnerability

We have a [unified security policy](https://github.com/CosmWasm/wasmd/blob/master/SECURITY.md)
for all CosmWasm-related repositories maintained by Confio.
You can [read it here](https://github.com/CosmWasm/wasmd/blob/master/SECURITY.md)
