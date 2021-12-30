# Changelog

## [Unreleased](https://github.com/CosmWasm/cw-plus/tree/HEAD)

[Full Changelog](https://github.com/CosmWasm/cw-plus/compare/v0.10.3...HEAD)

## [v0.10.3](https://github.com/CosmWasm/cw-plus/tree/v0.10.3) (2021-11-16)

**Implemented enhancements:**

- Deprecate IntKey [\#547](https://github.com/CosmWasm/cw-plus/pull/547) ([ueco-jb](https://github.com/ueco-jb))
- Implement WasmQuery::ContractInfo [\#554](https://github.com/CosmWasm/cw-plus/pull/554) ([ethanfrey](https://github.com/ethanfrey))

**Fixed bugs:**

- Fix min threshold and vote power bugs in cw3-flex-multisig [\#527](https://github.com/CosmWasm/cw-plus/issues/527)

**Closed issues:**

- "env.sender" in README of cw20 [\#539](https://github.com/CosmWasm/cw-plus/issues/539)
- Migrate example [\#511](https://github.com/CosmWasm/cw-plus/issues/511)
- Semver parsing / comparison [\#510](https://github.com/CosmWasm/cw-plus/issues/510)
- Example of parsing SubMessage data field [\#509](https://github.com/CosmWasm/cw-plus/issues/509)
- Deprecate `IntKey` [\#472](https://github.com/CosmWasm/cw-plus/issues/472)

**Merged pull requests:**

- Update cw1-subkeys/helpers.ts file to work with cosmjs/cli v0.26 [\#546](https://github.com/CosmWasm/cw-plus/pull/546) ([findolor](https://github.com/findolor))
- Fix cw20 readme [\#544](https://github.com/CosmWasm/cw-plus/pull/544) ([loloicci](https://github.com/loloicci))
- Revert "Update helper version and refactor based on new base helper" [\#538](https://github.com/CosmWasm/cw-plus/pull/538) ([findolor](https://github.com/findolor))
- Update cw1-subkeys/helpers.ts version and refactor based on base-helper.ts [\#537](https://github.com/CosmWasm/cw-plus/pull/537) ([findolor](https://github.com/findolor))
- Refactor cw20-base/helpers.ts based on base-helper.ts [\#536](https://github.com/CosmWasm/cw-plus/pull/536) ([findolor](https://github.com/findolor))
- Add base helper for contracts [\#535](https://github.com/CosmWasm/cw-plus/pull/535) ([findolor](https://github.com/findolor))
- Fix min threshold in cw3-flex-multisig [\#528](https://github.com/CosmWasm/cw-plus/pull/528) ([ueco-jb](https://github.com/ueco-jb))
- cw1-subkeys: Migration example [\#525](https://github.com/CosmWasm/cw-plus/pull/525) ([hashedone](https://github.com/hashedone))

## [v0.10.2](https://github.com/CosmWasm/cw-plus/tree/v0.10.2) (2021-11-03)

**Closed issues:**

- Multitest has errors with reply data [\#516](https://github.com/CosmWasm/cw-plus/issues/516)

**Merged pull requests:**

- cw1-whitelist-ng: Slight messages parsing improvement [\#523](https://github.com/CosmWasm/cw-plus/pull/523) ([hashedone](https://github.com/hashedone))
- ics20: Handle send errors with reply [\#520](https://github.com/CosmWasm/cw-plus/pull/520) ([ethanfrey](https://github.com/ethanfrey))
- Proper execute responses [\#519](https://github.com/CosmWasm/cw-plus/pull/519) ([ethanfrey](https://github.com/ethanfrey))
- Publish MsgInstantiate / Execute responses [\#518](https://github.com/CosmWasm/cw-plus/pull/518) ([maurolacy](https://github.com/maurolacy))
- Fix instaniate reply data [\#517](https://github.com/CosmWasm/cw-plus/pull/517) ([ethanfrey](https://github.com/ethanfrey))
- Use protobuf de helpers [\#515](https://github.com/CosmWasm/cw-plus/pull/515) ([maurolacy](https://github.com/maurolacy))
- Add tests for the claims controller [\#514](https://github.com/CosmWasm/cw-plus/pull/514) ([sgoya](https://github.com/sgoya))
- Implement cw3-flex-multisig helper [\#479](https://github.com/CosmWasm/cw-plus/pull/479) ([orkunkl](https://github.com/orkunkl))

## [v0.10.1](https://github.com/CosmWasm/cw-plus/tree/v0.10.1) (2021-10-26)

[Full Changelog](https://github.com/CosmWasm/cw-plus/compare/v0.10.0...v0.10.1)

**Closed issues:**

- Reimplement cw1-whitelist contract in terms of semantic structures [\#494](https://github.com/CosmWasm/cw-plus/issues/494)
- Helper transfer method failed to execute message [\#492](https://github.com/CosmWasm/cw-plus/issues/492)
- Add helpers for parsing the protobuf MsgInstantiate and MsgExecute responses [\#480](https://github.com/CosmWasm/cw-plus/issues/480)

**Merged pull requests:**

- Prepare 0.10.1 release [\#513](https://github.com/CosmWasm/cw-plus/pull/513) ([ethanfrey](https://github.com/ethanfrey))
- Added cw1-whitelist-ng to CI [\#512](https://github.com/CosmWasm/cw-plus/pull/512) ([hashedone](https://github.com/hashedone))
- cw1-subkeys-ng: Additional follow up improvements [\#506](https://github.com/CosmWasm/cw-plus/pull/506) ([hashedone](https://github.com/hashedone))
- Parse reply helpers [\#502](https://github.com/CosmWasm/cw-plus/pull/502) ([maurolacy](https://github.com/maurolacy))
- cw1-whitelist-ng: Contract implementation in terms of semantical structures [\#499](https://github.com/CosmWasm/cw-plus/pull/499) ([hashedone](https://github.com/hashedone))
- range\_de for IndexMap [\#498](https://github.com/CosmWasm/cw-plus/pull/498) ([uint](https://github.com/uint))
- Implement range\_de for SnapshotMap [\#497](https://github.com/CosmWasm/cw-plus/pull/497) ([uint](https://github.com/uint))
- Fix publish script [\#486](https://github.com/CosmWasm/cw-plus/pull/486) ([ethanfrey](https://github.com/ethanfrey))
- Implement cw4-group typescript helper [\#476](https://github.com/CosmWasm/cw-plus/pull/476) ([orkunkl](https://github.com/orkunkl))

## [v0.10.0](https://github.com/CosmWasm/cw-plus/tree/v0.10.0) (2021-10-11)

[Full Changelog](https://github.com/CosmWasm/cw-plus/compare/v0.10.0-soon4...v0.10.0)

**Closed issues:**

- Question about `MultiIndex` [\#466](https://github.com/CosmWasm/cw-plus/issues/466)
- More multitest improvements [\#266](https://github.com/CosmWasm/cw-plus/issues/266)
- Update to cosmwasm v1.0.0-soon2 [\#473](https://github.com/CosmWasm/cw-plus/issues/473)
- Allow NFTs to include custom data [\#440](https://github.com/CosmWasm/cw-plus/issues/440)
- Refactor Admin cw-controller to better represent actual functionality [\#424](https://github.com/CosmWasm/cw-plus/issues/424)
- Implement `PrimaryKey` for `Timestamp` [\#419](https://github.com/CosmWasm/cw-plus/issues/419)
- storage-plus: Improve in-code documentation of map primitives, in particular `MultiIndex` [\#407](https://github.com/CosmWasm/cw-plus/issues/407)
- Remove use of dyn in multitest Router [\#404](https://github.com/CosmWasm/cw-plus/issues/404)
- Define generic multitest module [\#387](https://github.com/CosmWasm/cw-plus/issues/387)

**Merged pull requests:**

- Update CHANGELOG [\#485](https://github.com/CosmWasm/cw-plus/pull/485) ([ethanfrey](https://github.com/ethanfrey))
- Release 0.10.0 [\#483](https://github.com/CosmWasm/cw-plus/pull/483) ([ethanfrey](https://github.com/ethanfrey))
- Upgrade CosmWasm to 1.0.0-beta [\#482](https://github.com/CosmWasm/cw-plus/pull/482) ([webmaster128](https://github.com/webmaster128))
- Full deserialization for `range` [\#432](https://github.com/CosmWasm/cw-plus/pull/432) ([maurolacy](https://github.com/maurolacy))

## [v0.10.0-soon4](https://github.com/CosmWasm/cw-plus/tree/v0.10.0-soon4) (2021-10-07)

[Full Changelog](https://github.com/CosmWasm/cw-plus/compare/v0.10.0-soon3...v0.10.0-soon4)

**Fixed bugs:**

- Fix improper assert\_matches usage [\#459](https://github.com/CosmWasm/cw-plus/pull/459) ([ueco-jb](https://github.com/ueco-jb))

**Closed issues:**

- Update to cosmwasm v1.0.0-soon2 [\#473](https://github.com/CosmWasm/cw-plus/issues/473)
- Add ensure! macro [\#468](https://github.com/CosmWasm/cw-plus/issues/468)
- Better return values from range/prefix [\#198](https://github.com/CosmWasm/cw-plus/issues/198)

**Merged pull requests:**

- Release v0.10.0-soon4 [\#477](https://github.com/CosmWasm/cw-plus/pull/477) ([ethanfrey](https://github.com/ethanfrey))
- Update to CosmWasm 1.0.0-soon2  [\#475](https://github.com/CosmWasm/cw-plus/pull/475) ([ethanfrey](https://github.com/ethanfrey))
- Allow error type conversions in ensure! and ensure\_eq! [\#474](https://github.com/CosmWasm/cw-plus/pull/474) ([webmaster128](https://github.com/webmaster128))
- Improve error handling / remove FIXMEs [\#470](https://github.com/CosmWasm/cw-plus/pull/470) ([maurolacy](https://github.com/maurolacy))
- Add ensure [\#469](https://github.com/CosmWasm/cw-plus/pull/469) ([ethanfrey](https://github.com/ethanfrey))
- Key deserializer improvements [\#467](https://github.com/CosmWasm/cw-plus/pull/467) ([maurolacy](https://github.com/maurolacy))
- Upgrade to cosmwasm/workspace-optimizer:0.12.3 [\#465](https://github.com/CosmWasm/cw-plus/pull/465) ([webmaster128](https://github.com/webmaster128))
- Prefix consolidation [\#439](https://github.com/CosmWasm/cw-plus/pull/439) ([maurolacy](https://github.com/maurolacy))
- Full deserialization for `range` [\#432](https://github.com/CosmWasm/cw-plus/pull/432) ([maurolacy](https://github.com/maurolacy))

## [v0.10.0-soon3](https://github.com/CosmWasm/cw-plus/tree/v0.10.0-soon3) (2021-09-29)

[Full Changelog](https://github.com/CosmWasm/cw-plus/compare/v0.10.0-soon2...v0.10.0-soon3)

**Merged pull requests:**

- Prepare release v0.10.0-soon3 [\#457](https://github.com/CosmWasm/cw-plus/pull/457) ([ethanfrey](https://github.com/ethanfrey))
- Expose essential multitest types [\#456](https://github.com/CosmWasm/cw-plus/pull/456) ([ethanfrey](https://github.com/ethanfrey))

## [v0.10.0-soon2](https://github.com/CosmWasm/cw-plus/tree/v0.10.0-soon2) (2021-09-28)

[Full Changelog](https://github.com/CosmWasm/cw-plus/compare/v0.9.1...v0.10.0-soon2)

**Merged pull requests:**

- Release 0.10.0-soon2 [\#455](https://github.com/CosmWasm/cw-plus/pull/455) ([ethanfrey](https://github.com/ethanfrey))
- Expose sudo powers on Router we give to Modules [\#453](https://github.com/CosmWasm/cw-plus/pull/453) ([ethanfrey](https://github.com/ethanfrey))
- Forward port 440 demo metadata extension [\#452](https://github.com/CosmWasm/cw-plus/pull/452) ([ethanfrey](https://github.com/ethanfrey))
- Forward port 440-customize-nft [\#451](https://github.com/CosmWasm/cw-plus/pull/451) ([ethanfrey](https://github.com/ethanfrey))

## [v0.9.1](https://github.com/CosmWasm/cw-plus/tree/v0.9.1) (2021-09-23)

[Full Changelog](https://github.com/CosmWasm/cw-plus/compare/v0.10.0-soon...v0.9.1)

**Closed issues:**

- Allow NFTs to include custom data [\#440](https://github.com/CosmWasm/cw-plus/issues/440)

## [v0.10.0-soon](https://github.com/CosmWasm/cw-plus/tree/v0.10.0-soon) (2021-09-22)

[Full Changelog](https://github.com/CosmWasm/cw-plus/compare/v0.9.0...v0.10.0-soon)

**Closed issues:**

- Contracts for Token Sale and Vesting period [\#444](https://github.com/CosmWasm/cw-plus/issues/444)
- small updates on storage-plus docs [\#435](https://github.com/CosmWasm/cw-plus/issues/435)
- Unintuitive behavior of range on multi-index [\#430](https://github.com/CosmWasm/cw-plus/issues/430)
- Upgrade to cosmwasm 1.0-soon [\#427](https://github.com/CosmWasm/cw-plus/issues/427)
- Refactor Admin cw-controller to better represent actual functionality [\#424](https://github.com/CosmWasm/cw-plus/issues/424)
- Add auto-changelog generator [\#421](https://github.com/CosmWasm/cw-plus/issues/421)
- Implement `PrimaryKey` for `Timestamp` [\#419](https://github.com/CosmWasm/cw-plus/issues/419)
- storage-plus: Improve in-code documentation of map primitives, in particular `MultiIndex` [\#407](https://github.com/CosmWasm/cw-plus/issues/407)
- Remove use of dyn in multitest Router [\#404](https://github.com/CosmWasm/cw-plus/issues/404)
- Define generic multitest module [\#387](https://github.com/CosmWasm/cw-plus/issues/387)
- Cw20 state key compatibity with previous versions  [\#346](https://github.com/CosmWasm/cw-plus/issues/346)
- Refactor cw20-base to use controller pattern [\#205](https://github.com/CosmWasm/cw-plus/issues/205)

**Merged pull requests:**

- Release 0.10.0-soon [\#448](https://github.com/CosmWasm/cw-plus/pull/448) ([ethanfrey](https://github.com/ethanfrey))
- Add proper prefix\_range helper when you want to iterate over the prefix space [\#446](https://github.com/CosmWasm/cw-plus/pull/446) ([ethanfrey](https://github.com/ethanfrey))
- Improve in-code documentation of map primitives [\#443](https://github.com/CosmWasm/cw-plus/pull/443) ([ueco-jb](https://github.com/ueco-jb))
- Small storage-plus docs update [\#442](https://github.com/CosmWasm/cw-plus/pull/442) ([hashedone](https://github.com/hashedone))
- Upgrade to cosmwasm 1.0.0-soon [\#441](https://github.com/CosmWasm/cw-plus/pull/441) ([ethanfrey](https://github.com/ethanfrey))
- Test storage-plus with iterator disabled [\#438](https://github.com/CosmWasm/cw-plus/pull/438) ([ethanfrey](https://github.com/ethanfrey))
- Multitest module query [\#437](https://github.com/CosmWasm/cw-plus/pull/437) ([ethanfrey](https://github.com/ethanfrey))
- Range with no prefix support [\#433](https://github.com/CosmWasm/cw-plus/pull/433) ([maurolacy](https://github.com/maurolacy))
- Added implementation of timestamp key [\#431](https://github.com/CosmWasm/cw-plus/pull/431) ([hashedone](https://github.com/hashedone))
- Update changelog 2 [\#428](https://github.com/CosmWasm/cw-plus/pull/428) ([maurolacy](https://github.com/maurolacy))
- Add automatically generated changelog [\#426](https://github.com/CosmWasm/cw-plus/pull/426) ([ueco-jb](https://github.com/ueco-jb))
- Generic module types [\#425](https://github.com/CosmWasm/cw-plus/pull/425) ([ethanfrey](https://github.com/ethanfrey))
- Simplify router args [\#422](https://github.com/CosmWasm/cw-plus/pull/422) ([ethanfrey](https://github.com/ethanfrey))
- Snapshot item 2 [\#418](https://github.com/CosmWasm/cw-plus/pull/418) ([maurolacy](https://github.com/maurolacy))
- Removing dyn from Router [\#410](https://github.com/CosmWasm/cw-plus/pull/410) ([hashedone](https://github.com/hashedone))

## [v0.9.0](https://github.com/CosmWasm/cw-plus/tree/v0.9.0) (2021-09-14)

[Full Changelog](https://github.com/CosmWasm/cw-plus/compare/v0.8.1...v0.9.0)

**Implemented enhancements:**

- Move from using unsound `from_utf8_unchecked` to safe `from_utf8` forwarding error [\#393](https://github.com/CosmWasm/cw-plus/issues/393)
- Raw Query: make usage simpler and visible [\#325](https://github.com/CosmWasm/cw-plus/issues/325)
- Consider replacing `String` errors with `anyhow::Error` in interfaces [\#361](https://github.com/CosmWasm/cw-plus/issues/361)

**Closed issues:**

- Generalize controllers [\#408](https://github.com/CosmWasm/cw-plus/issues/408)
- Extend `Claim` to be able to index claims via expiration time [\#405](https://github.com/CosmWasm/cw-plus/issues/405)
- Update Cargo.toml files to reference new repo name [\#403](https://github.com/CosmWasm/cw-plus/issues/403)
- Test execute on cw1-whitelist contract [\#400](https://github.com/CosmWasm/cw-plus/issues/400)
- Accept &QuerierWrapper not &Querier in helpers [\#390](https://github.com/CosmWasm/cw-plus/issues/390)
- Use builder pattern for App init [\#388](https://github.com/CosmWasm/cw-plus/issues/388)
- Idea: item query helper storage helper [\#376](https://github.com/CosmWasm/cw-plus/issues/376)
- Why you use `Addr` as keys in Maps? [\#295](https://github.com/CosmWasm/cw-plus/issues/295)
- Add SnapshotItem to storage-plus [\#193](https://github.com/CosmWasm/cw-plus/issues/193)
- Fix lifetime of MultiIndex/UniqueIndex to be able to accept &str [\#232](https://github.com/CosmWasm/cw-plus/issues/232)
- Unify multisig structs Member and VoterResponse [\#151](https://github.com/CosmWasm/cw-plus/issues/151)

**Merged pull requests:**

- admin and hooks return Response\<C\> in execute\_\* [\#417](https://github.com/CosmWasm/cw-plus/pull/417) ([ethanfrey](https://github.com/ethanfrey))
- Release 0.9.0 [\#416](https://github.com/CosmWasm/cw-plus/pull/416) ([ethanfrey](https://github.com/ethanfrey))
- Add send and sendFrom to cw20-base helpers.ts [\#415](https://github.com/CosmWasm/cw-plus/pull/415) ([orkunkl](https://github.com/orkunkl))
- Add doc entry about key usage in maps [\#413](https://github.com/CosmWasm/cw-plus/pull/413) ([maurolacy](https://github.com/maurolacy))
- Add query helpers to Item and Map and use them in cw4 helpers [\#412](https://github.com/CosmWasm/cw-plus/pull/412) ([ethanfrey](https://github.com/ethanfrey))
- Update Cargo.toml files to reference new repo name [\#411](https://github.com/CosmWasm/cw-plus/pull/411) ([ueco-jb](https://github.com/ueco-jb))
- Snapshot item [\#409](https://github.com/CosmWasm/cw-plus/pull/409) ([maurolacy](https://github.com/maurolacy))
- cw20-base: upgrade helper.ts to cosmjs 0.26.0 [\#406](https://github.com/CosmWasm/cw-plus/pull/406) ([spacepotahto](https://github.com/spacepotahto))
- CW1-whitelist execute multitest [\#402](https://github.com/CosmWasm/cw-plus/pull/402) ([ueco-jb](https://github.com/ueco-jb))
- Implementing all messages handling in mutlitest App [\#398](https://github.com/CosmWasm/cw-plus/pull/398) ([hashedone](https://github.com/hashedone))
- Make it easier to assert events on reply statements [\#395](https://github.com/CosmWasm/cw-plus/pull/395) ([ethanfrey](https://github.com/ethanfrey))
- Add helpers to check events [\#392](https://github.com/CosmWasm/cw-plus/pull/392) ([ethanfrey](https://github.com/ethanfrey))
- Switching from String to anyhow::Error for error type in multi-test [\#389](https://github.com/CosmWasm/cw-plus/pull/389) ([hashedone](https://github.com/hashedone))

## [v0.8.1](https://github.com/CosmWasm/cw-plus/tree/v0.8.1) (2021-08-26)

[Full Changelog](https://github.com/CosmWasm/cw-plus/compare/v0.8.0...v0.8.1)

**Implemented enhancements:**

- Consider replacing `String` errors with `anyhow::Error` in interfaces [\#361](https://github.com/CosmWasm/cw-plus/issues/361)

**Closed issues:**

- Fix lifetime of MultiIndex/UniqueIndex to be able to accept &str [\#232](https://github.com/CosmWasm/cw-plus/issues/232)
- Unify multisig structs Member and VoterResponse [\#151](https://github.com/CosmWasm/cw-plus/issues/151)
- Add exhaustive checks for errors in contracts [\#105](https://github.com/CosmWasm/cw-plus/issues/105)

## [v0.8.0](https://github.com/CosmWasm/cw-plus/tree/v0.8.0) (2021-08-10)

[Full Changelog](https://github.com/CosmWasm/cw-plus/compare/v0.8.0-rc3...v0.8.0)

**Closed issues:**

- Upgrade CosmWasm to 0.16.0 [\#377](https://github.com/CosmWasm/cw-plus/issues/377)
- Upgrade rust to 1.53 [\#372](https://github.com/CosmWasm/cw-plus/issues/372)
- Implement cw20 logo spec for cw20-base [\#371](https://github.com/CosmWasm/cw-plus/issues/371)
- multi-test: ensure event handling matches wasmd 0.18 implementation [\#348](https://github.com/CosmWasm/cw-plus/issues/348)

**Merged pull requests:**

- Added some missing traits on messages of cw20-base [\#386](https://github.com/CosmWasm/cw-plus/pull/386) ([hashedone](https://github.com/hashedone))

## [v0.8.0-rc3](https://github.com/CosmWasm/cw-plus/tree/v0.8.0-rc3) (2021-08-10)

[Full Changelog](https://github.com/CosmWasm/cw-plus/compare/v0.8.0-rc2...v0.8.0-rc3)

**Closed issues:**

- multi-test: ensure event handling matches wasmd 0.18 implementation [\#348](https://github.com/CosmWasm/cw-plus/issues/348)

**Merged pull requests:**

- Corrected submessage data response handling in multi-test [\#385](https://github.com/CosmWasm/cw-plus/pull/385) ([hashedone](https://github.com/hashedone))
- Document submsg data failures and fix them [\#383](https://github.com/CosmWasm/cw-plus/pull/383) ([ethanfrey](https://github.com/ethanfrey))
- Adaptors for all contracts and entry points from Empty -\> C [\#382](https://github.com/CosmWasm/cw-plus/pull/382) ([ethanfrey](https://github.com/ethanfrey))
- Multitest events match wasmd [\#380](https://github.com/CosmWasm/cw-plus/pull/380) ([ethanfrey](https://github.com/ethanfrey))

## [v0.8.0-rc2](https://github.com/CosmWasm/cw-plus/tree/v0.8.0-rc2) (2021-08-05)

[Full Changelog](https://github.com/CosmWasm/cw-plus/compare/v0.8.0-rc1...v0.8.0-rc2)

**Closed issues:**

- Upgrade CosmWasm to 0.16.0 [\#377](https://github.com/CosmWasm/cw-plus/issues/377)
- Upgrade rust to 1.53 [\#372](https://github.com/CosmWasm/cw-plus/issues/372)
- Implement cw20 logo spec for cw20-base [\#371](https://github.com/CosmWasm/cw-plus/issues/371)
- multi-test: Pass in API access to BankKeeper [\#353](https://github.com/CosmWasm/cw-plus/issues/353)
- multi-test: data handling with replies [\#352](https://github.com/CosmWasm/cw-plus/issues/352)
- multi-test: Add migrate support [\#351](https://github.com/CosmWasm/cw-plus/issues/351)
- multitest: Ensure Warm sent funds visible to querier [\#347](https://github.com/CosmWasm/cw-plus/issues/347)
- multitest: Enforce validity checks for returned items [\#341](https://github.com/CosmWasm/cw-plus/issues/341)

**Merged pull requests:**

- Update to Rust 1.53 [\#379](https://github.com/CosmWasm/cw-plus/pull/379) ([ethanfrey](https://github.com/ethanfrey))
- Upgrade to cosmwasm 0.16 [\#378](https://github.com/CosmWasm/cw-plus/pull/378) ([ethanfrey](https://github.com/ethanfrey))
- Marketing info for cw20-base contract [\#375](https://github.com/CosmWasm/cw-plus/pull/375) ([hashedone](https://github.com/hashedone))
- cw20-merkle-airdrop: change hashing to sha256 [\#374](https://github.com/CosmWasm/cw-plus/pull/374) ([orkunkl](https://github.com/orkunkl))
- Responses validation in multi-test [\#373](https://github.com/CosmWasm/cw-plus/pull/373) ([hashedone](https://github.com/hashedone))
- Cw20 logo spec [\#370](https://github.com/CosmWasm/cw-plus/pull/370) ([ethanfrey](https://github.com/ethanfrey))
- Properly handling data in submessages in multi-test [\#369](https://github.com/CosmWasm/cw-plus/pull/369) ([hashedone](https://github.com/hashedone))
- Abstracting API out of tests internals so it is clearly owned by `App` [\#368](https://github.com/CosmWasm/cw-plus/pull/368) ([hashedone](https://github.com/hashedone))
- Storage plus doc correction [\#367](https://github.com/CosmWasm/cw-plus/pull/367) ([hashedone](https://github.com/hashedone))
- Multitest migrate support [\#366](https://github.com/CosmWasm/cw-plus/pull/366) ([ethanfrey](https://github.com/ethanfrey))
- Reorganizations of contracts in `multi-test::test_utils` [\#365](https://github.com/CosmWasm/cw-plus/pull/365) ([hashedone](https://github.com/hashedone))
- Implement cw20-merkle-airdrop [\#364](https://github.com/CosmWasm/cw-plus/pull/364) ([orkunkl](https://github.com/orkunkl))
- Testing sent founds visibility in multi-test [\#363](https://github.com/CosmWasm/cw-plus/pull/363) ([hashedone](https://github.com/hashedone))

## [v0.8.0-rc1](https://github.com/CosmWasm/cw-plus/tree/v0.8.0-rc1) (2021-07-29)

[Full Changelog](https://github.com/CosmWasm/cw-plus/compare/v0.7.0...v0.8.0-rc1)

**Closed issues:**

- Lack of `overflow-checks=true` in contracts [\#358](https://github.com/CosmWasm/cw-plus/issues/358)
- multi-test: Store more data in ContractData [\#350](https://github.com/CosmWasm/cw-plus/issues/350)
- multi-test: cleaner use of transactions [\#349](https://github.com/CosmWasm/cw-plus/issues/349)
- Deprecate `pks()` [\#344](https://github.com/CosmWasm/cw-plus/issues/344)
- Why cw20-base do not care about minting cap? [\#339](https://github.com/CosmWasm/cw-plus/issues/339)
- Upgrade to cosmwasm 0.16 [\#333](https://github.com/CosmWasm/cw-plus/issues/333)
- multi-test: Bank returns realistic events [\#329](https://github.com/CosmWasm/cw-plus/issues/329)
- storage-plus: Need better docs and examples for IndexedMap [\#327](https://github.com/CosmWasm/cw-plus/issues/327)
- Improve `PkOwned` usability through `From` / `Into` [\#234](https://github.com/CosmWasm/cw-plus/issues/234)
- Add ContractAddr generic helper [\#153](https://github.com/CosmWasm/cw-plus/issues/153)
- Brainstorm: cw-storage-plus support when key can be derived from stored object [\#120](https://github.com/CosmWasm/cw-plus/issues/120)

**Merged pull requests:**

- Extend `ContractData` in multi-test [\#360](https://github.com/CosmWasm/cw-plus/pull/360) ([hashedone](https://github.com/hashedone))
- Add transactional helper [\#357](https://github.com/CosmWasm/cw-plus/pull/357) ([ethanfrey](https://github.com/ethanfrey))
- Implemented expiration for cw1-subkeys contract [\#356](https://github.com/CosmWasm/cw-plus/pull/356) ([hashedone](https://github.com/hashedone))
- Clarify how cw20 minting is supposed to work [\#355](https://github.com/CosmWasm/cw-plus/pull/355) ([ethanfrey](https://github.com/ethanfrey))
- Permission bugs corrected in cw1-subkeys [\#354](https://github.com/CosmWasm/cw-plus/pull/354) ([hashedone](https://github.com/hashedone))
- Deprecate pks [\#345](https://github.com/CosmWasm/cw-plus/pull/345) ([maurolacy](https://github.com/maurolacy))
- Refactor of cw1-whitelist unit tests [\#343](https://github.com/CosmWasm/cw-plus/pull/343) ([hashedone](https://github.com/hashedone))
- Cw721 token indexes refactor [\#342](https://github.com/CosmWasm/cw-plus/pull/342) ([maurolacy](https://github.com/maurolacy))
- Indexed map docs [\#340](https://github.com/CosmWasm/cw-plus/pull/340) ([maurolacy](https://github.com/maurolacy))
- Cosmwasm 0.16 [\#338](https://github.com/CosmWasm/cw-plus/pull/338) ([uint](https://github.com/uint))
- Multitest bank events [\#337](https://github.com/CosmWasm/cw-plus/pull/337) ([ethanfrey](https://github.com/ethanfrey))
- Fix clippy +1.53.0 warnings [\#336](https://github.com/CosmWasm/cw-plus/pull/336) ([maurolacy](https://github.com/maurolacy))
- Simplify multitest storage [\#335](https://github.com/CosmWasm/cw-plus/pull/335) ([ethanfrey](https://github.com/ethanfrey))
- Contract builders [\#334](https://github.com/CosmWasm/cw-plus/pull/334) ([ethanfrey](https://github.com/ethanfrey))
- Update to cw schema 0.15.0 [\#332](https://github.com/CosmWasm/cw-plus/pull/332) ([maurolacy](https://github.com/maurolacy))

## [v0.7.0](https://github.com/CosmWasm/cw-plus/tree/v0.7.0) (2021-07-14)

[Full Changelog](https://github.com/CosmWasm/cw-plus/compare/v0.6.2...v0.7.0)

**Closed issues:**

- multi-test: Proper protobuf-encoded data for init [\#330](https://github.com/CosmWasm/cw-plus/issues/330)
- Proper event/data handling on reply in multitest [\#326](https://github.com/CosmWasm/cw-plus/issues/326)
- Messages differ for cw20 & cw20\_base [\#320](https://github.com/CosmWasm/cw-plus/issues/320)
- Upgrade cw20-staking to cw 15 [\#312](https://github.com/CosmWasm/cw-plus/issues/312)
- Uprade cw20-ics20 to cw 0.15 [\#311](https://github.com/CosmWasm/cw-plus/issues/311)
- Upgrade cw20-escrow to 0.15 [\#309](https://github.com/CosmWasm/cw-plus/issues/309)
- Upgrade cw20-bonding to 0.15 [\#307](https://github.com/CosmWasm/cw-plus/issues/307)
- cw1-subkeys [\#305](https://github.com/CosmWasm/cw-plus/issues/305)
- Upgrade cw20-base to 0.15 [\#302](https://github.com/CosmWasm/cw-plus/issues/302)
- Upgrade cosmwasm-plus contracts [\#300](https://github.com/CosmWasm/cw-plus/issues/300)
- Upgrade to CosmWasm 0.15 [\#298](https://github.com/CosmWasm/cw-plus/issues/298)
- Difference between native and cw20 tokens [\#297](https://github.com/CosmWasm/cw-plus/issues/297)
- Ensure all cw20 sends use `Binary` not `Option<Binary>` [\#296](https://github.com/CosmWasm/cw-plus/issues/296)
- Add existence helpers to cw-storage-plus [\#261](https://github.com/CosmWasm/cw-plus/issues/261)
- Support submessages in multitest [\#259](https://github.com/CosmWasm/cw-plus/issues/259)
- Build uniswap contract [\#7](https://github.com/CosmWasm/cw-plus/issues/7)

**Merged pull requests:**

- Use prost to create and parse proper InstantiateData [\#331](https://github.com/CosmWasm/cw-plus/pull/331) ([ethanfrey](https://github.com/ethanfrey))
- Reorg submessage [\#328](https://github.com/CosmWasm/cw-plus/pull/328) ([ethanfrey](https://github.com/ethanfrey))
- Cleanup multitest [\#324](https://github.com/CosmWasm/cw-plus/pull/324) ([ethanfrey](https://github.com/ethanfrey))
- Support submessages in multitest [\#323](https://github.com/CosmWasm/cw-plus/pull/323) ([ethanfrey](https://github.com/ethanfrey))
- Add has to Path and Map [\#322](https://github.com/CosmWasm/cw-plus/pull/322) ([ethanfrey](https://github.com/ethanfrey))
- Make receiver msg non-optional in cw20 contracts [\#321](https://github.com/CosmWasm/cw-plus/pull/321) ([ethanfrey](https://github.com/ethanfrey))
- Migrate contracts to 0.15.0 [\#318](https://github.com/CosmWasm/cw-plus/pull/318) ([orkunkl](https://github.com/orkunkl))
- Update remaining contracts to cosmwasm 0.15, fix problems [\#317](https://github.com/CosmWasm/cw-plus/pull/317) ([uint](https://github.com/uint))
- fix address range functions [\#316](https://github.com/CosmWasm/cw-plus/pull/316) ([pronvis](https://github.com/pronvis))
- Upgrade cw3 contracts and `cw4-group` [\#315](https://github.com/CosmWasm/cw-plus/pull/315) ([uint](https://github.com/uint))
- cw1155-base: upgrade cosmwasm-std to 0.15 [\#314](https://github.com/CosmWasm/cw-plus/pull/314) ([uint](https://github.com/uint))
- cw20-staking: Upgrade cw 0.15 [\#313](https://github.com/CosmWasm/cw-plus/pull/313) ([orkunkl](https://github.com/orkunkl))
- cw20-escrow: Upgrade to 0.15 [\#310](https://github.com/CosmWasm/cw-plus/pull/310) ([orkunkl](https://github.com/orkunkl))
- cw20-bonding: Upgrade to 0.15  [\#308](https://github.com/CosmWasm/cw-plus/pull/308) ([orkunkl](https://github.com/orkunkl))
- Update package schemas; fix linting errors [\#306](https://github.com/CosmWasm/cw-plus/pull/306) ([orkunkl](https://github.com/orkunkl))
- cw20-base: Upgrade to cw 0.15 [\#304](https://github.com/CosmWasm/cw-plus/pull/304) ([orkunkl](https://github.com/orkunkl))
- Upgrade cw1 contracts [\#303](https://github.com/CosmWasm/cw-plus/pull/303) ([uint](https://github.com/uint))
- Upgrade packages to cosmwasm 0.15.0 [\#301](https://github.com/CosmWasm/cw-plus/pull/301) ([uint](https://github.com/uint))
- cw20-base: upgrade helper.ts to cosmjs 0.25 [\#248](https://github.com/CosmWasm/cw-plus/pull/248) ([orkunkl](https://github.com/orkunkl))

## [v0.6.2](https://github.com/CosmWasm/cw-plus/tree/v0.6.2) (2021-06-23)

[Full Changelog](https://github.com/CosmWasm/cw-plus/compare/v0.6.1...v0.6.2)

**Merged pull requests:**

- Extend the allowed names and symbols for cw20-base [\#299](https://github.com/CosmWasm/cw-plus/pull/299) ([ethanfrey](https://github.com/ethanfrey))
- Implement PrimaryKey and Prefixer for String [\#294](https://github.com/CosmWasm/cw-plus/pull/294) ([ethanfrey](https://github.com/ethanfrey))

## [v0.6.1](https://github.com/CosmWasm/cw-plus/tree/v0.6.1) (2021-05-19)

[Full Changelog](https://github.com/CosmWasm/cw-plus/compare/v0.6.0...v0.6.1)

**Closed issues:**

- Expose contract errors [\#292](https://github.com/CosmWasm/cw-plus/issues/292)

**Merged pull requests:**

- Expose contract components [\#293](https://github.com/CosmWasm/cw-plus/pull/293) ([orkunkl](https://github.com/orkunkl))

## [v0.6.0](https://github.com/CosmWasm/cw-plus/tree/v0.6.0) (2021-05-03)

[Full Changelog](https://github.com/CosmWasm/cw-plus/compare/v0.6.0-beta3...v0.6.0)

**Closed issues:**

- Improve on indexed maps primary key / index keys helpers [\#277](https://github.com/CosmWasm/cw-plus/issues/277)
- CW20 example contract which 1-to-1 mapped to native token [\#276](https://github.com/CosmWasm/cw-plus/issues/276)
- Implement `PrimaryKey` for `HumanAddr` [\#256](https://github.com/CosmWasm/cw-plus/issues/256)
- Storage-plus: See if we can simplify PkOwned to Vec\<u8\> [\#199](https://github.com/CosmWasm/cw-plus/issues/199)

**Merged pull requests:**

- Clarify index\_key\(\) range\(\) vs prefix\(\) behaviour [\#291](https://github.com/CosmWasm/cw-plus/pull/291) ([maurolacy](https://github.com/maurolacy))
- Pkowned to vec u8 [\#290](https://github.com/CosmWasm/cw-plus/pull/290) ([maurolacy](https://github.com/maurolacy))
- Update to CosmWasm v0.14.0 [\#289](https://github.com/CosmWasm/cw-plus/pull/289) ([ethanfrey](https://github.com/ethanfrey))
- Primary key / index key helpers [\#288](https://github.com/CosmWasm/cw-plus/pull/288) ([maurolacy](https://github.com/maurolacy))

## [v0.6.0-beta3](https://github.com/CosmWasm/cw-plus/tree/v0.6.0-beta3) (2021-04-28)

[Full Changelog](https://github.com/CosmWasm/cw-plus/compare/v0.6.0-beta2...v0.6.0-beta3)

**Closed issues:**

- Make message required in Cw20ReceiveMsg [\#283](https://github.com/CosmWasm/cw-plus/issues/283)
- `Sudo` over no `new_with_sudo` contract wrapper error message [\#278](https://github.com/CosmWasm/cw-plus/issues/278)
- build\_and\_upload\_contract CI job fails [\#273](https://github.com/CosmWasm/cw-plus/issues/273)
- Add cw20 support to token-weighted group [\#143](https://github.com/CosmWasm/cw-plus/issues/143)

**Merged pull requests:**

- Cosmwasm beta5 [\#287](https://github.com/CosmWasm/cw-plus/pull/287) ([ethanfrey](https://github.com/ethanfrey))
- Cw20ReceiveMsg msg field [\#286](https://github.com/CosmWasm/cw-plus/pull/286) ([maurolacy](https://github.com/maurolacy))
- Fix ci contract build [\#285](https://github.com/CosmWasm/cw-plus/pull/285) ([ethanfrey](https://github.com/ethanfrey))
- Use Cw20 token in cw4-stake [\#282](https://github.com/CosmWasm/cw-plus/pull/282) ([ethanfrey](https://github.com/ethanfrey))
- Avoid the need for Any type by using Empty as message type and String as error type [\#281](https://github.com/CosmWasm/cw-plus/pull/281) ([webmaster128](https://github.com/webmaster128))
- Update to 0.14.0 beta4 [\#280](https://github.com/CosmWasm/cw-plus/pull/280) ([maurolacy](https://github.com/maurolacy))
- Better error message with missing sudo \(no parse error\) [\#279](https://github.com/CosmWasm/cw-plus/pull/279) ([ethanfrey](https://github.com/ethanfrey))

## [v0.6.0-beta2](https://github.com/CosmWasm/cw-plus/tree/v0.6.0-beta2) (2021-04-19)

[Full Changelog](https://github.com/CosmWasm/cw-plus/compare/v0.6.0-beta1...v0.6.0-beta2)

**Closed issues:**

- Add secondary index support to SnapshotMap. [\#255](https://github.com/CosmWasm/cw-plus/issues/255)

**Merged pull requests:**

- Indexed snapshot. Expose primary methods [\#275](https://github.com/CosmWasm/cw-plus/pull/275) ([maurolacy](https://github.com/maurolacy))
- Indexed snapshot map [\#271](https://github.com/CosmWasm/cw-plus/pull/271) ([maurolacy](https://github.com/maurolacy))
- Run clippy on test code [\#270](https://github.com/CosmWasm/cw-plus/pull/270) ([webmaster128](https://github.com/webmaster128))

## [v0.6.0-beta1](https://github.com/CosmWasm/cw-plus/tree/v0.6.0-beta1) (2021-04-13)

[Full Changelog](https://github.com/CosmWasm/cw-plus/compare/v0.6.0-alpha3...v0.6.0-beta1)

**Closed issues:**

- Update to cosmwasm v0.14.0-beta3 [\#268](https://github.com/CosmWasm/cw-plus/issues/268)
- My [\#263](https://github.com/CosmWasm/cw-plus/issues/263)

**Merged pull requests:**

- Bump dependency to cosmasm v0.14.0-beta3 [\#269](https://github.com/CosmWasm/cw-plus/pull/269) ([ethanfrey](https://github.com/ethanfrey))
- Remove unused PrimaryKey::parse\_key [\#267](https://github.com/CosmWasm/cw-plus/pull/267) ([webmaster128](https://github.com/webmaster128))
- Use workspace-optimizer:0.11.0 [\#262](https://github.com/CosmWasm/cw-plus/pull/262) ([webmaster128](https://github.com/webmaster128))
- Update cosmwasm-std [\#260](https://github.com/CosmWasm/cw-plus/pull/260) ([yihuang](https://github.com/yihuang))
- implement demo cw1155 contract [\#251](https://github.com/CosmWasm/cw-plus/pull/251) ([yihuang](https://github.com/yihuang))

## [v0.6.0-alpha3](https://github.com/CosmWasm/cw-plus/tree/v0.6.0-alpha3) (2021-04-01)

[Full Changelog](https://github.com/CosmWasm/cw-plus/compare/v0.6.0-alpha2...v0.6.0-alpha3)

**Merged pull requests:**

- More multitest improvements [\#258](https://github.com/CosmWasm/cw-plus/pull/258) ([ethanfrey](https://github.com/ethanfrey))

## [v0.6.0-alpha2](https://github.com/CosmWasm/cw-plus/tree/v0.6.0-alpha2) (2021-04-01)

[Full Changelog](https://github.com/CosmWasm/cw-plus/compare/v0.6.0-alpha1...v0.6.0-alpha2)

**Closed issues:**

- Re-enable field\_reassign\_with\_default [\#252](https://github.com/CosmWasm/cw-plus/issues/252)
- No equivalent of ERC1155 standard [\#246](https://github.com/CosmWasm/cw-plus/issues/246)
- Rename HandleMsg to ExecuteMsg [\#236](https://github.com/CosmWasm/cw-plus/issues/236)
- Use \#\[entry\_point\] macro in contracts [\#230](https://github.com/CosmWasm/cw-plus/issues/230)
- Support PartialEq on error [\#179](https://github.com/CosmWasm/cw-plus/issues/179)

**Merged pull requests:**

- Enhance multi test [\#257](https://github.com/CosmWasm/cw-plus/pull/257) ([ethanfrey](https://github.com/ethanfrey))
- Update to Rust v1.51.0 [\#254](https://github.com/CosmWasm/cw-plus/pull/254) ([maurolacy](https://github.com/maurolacy))
- PartialEq for errors [\#253](https://github.com/CosmWasm/cw-plus/pull/253) ([maurolacy](https://github.com/maurolacy))
- Handle msg to execute msg [\#250](https://github.com/CosmWasm/cw-plus/pull/250) ([maurolacy](https://github.com/maurolacy))
- Migrate to entry\_point macro [\#249](https://github.com/CosmWasm/cw-plus/pull/249) ([maurolacy](https://github.com/maurolacy))
- Add cw1155 specification [\#247](https://github.com/CosmWasm/cw-plus/pull/247) ([yihuang](https://github.com/yihuang))

## [v0.6.0-alpha1](https://github.com/CosmWasm/cw-plus/tree/v0.6.0-alpha1) (2021-03-12)

[Full Changelog](https://github.com/CosmWasm/cw-plus/compare/v0.5.0...v0.6.0-alpha1)

**Implemented enhancements:**

- Add contract sanity checking during build / release [\#228](https://github.com/CosmWasm/cw-plus/issues/228)

**Closed issues:**

- Update to CosmWasm v0.14.0-beta1 [\#242](https://github.com/CosmWasm/cw-plus/issues/242)
- Support life-timed references in `UniqueIndex` and `MultiIndex` keys [\#233](https://github.com/CosmWasm/cw-plus/issues/233)
- Write cw20-ics20 ibc enabled contract [\#231](https://github.com/CosmWasm/cw-plus/issues/231)
- Upgrade to CosmWasm v0.14.0 [\#229](https://github.com/CosmWasm/cw-plus/issues/229)
- Fix / remove cw20-bonding floating point instructions [\#227](https://github.com/CosmWasm/cw-plus/issues/227)
- Add cw20-ics20 contract [\#226](https://github.com/CosmWasm/cw-plus/issues/226)
- Upgrade to CosmWasm 0.14 [\#225](https://github.com/CosmWasm/cw-plus/issues/225)
- Use entry\_point macro for contract entry-points [\#224](https://github.com/CosmWasm/cw-plus/issues/224)
- Upgrade Contracts to storage-plus [\#203](https://github.com/CosmWasm/cw-plus/issues/203)
- Support composite keys on secondary indexes \(multi-index\) [\#163](https://github.com/CosmWasm/cw-plus/issues/163)

**Merged pull requests:**

- Fix ics20 denom [\#244](https://github.com/CosmWasm/cw-plus/pull/244) ([ethanfrey](https://github.com/ethanfrey))
- Update to 0.14.0 beta1 [\#243](https://github.com/CosmWasm/cw-plus/pull/243) ([maurolacy](https://github.com/maurolacy))
- Upgrade cw1 to storage plus [\#241](https://github.com/CosmWasm/cw-plus/pull/241) ([ethanfrey](https://github.com/ethanfrey))
- Contract sanity checking [\#240](https://github.com/CosmWasm/cw-plus/pull/240) ([maurolacy](https://github.com/maurolacy))
- Converting cw20-\* contracts to use storage-plus [\#239](https://github.com/CosmWasm/cw-plus/pull/239) ([ethanfrey](https://github.com/ethanfrey))
- Create Contract to send cw20 tokens over ics20  [\#238](https://github.com/CosmWasm/cw-plus/pull/238) ([ethanfrey](https://github.com/ethanfrey))
- Cw20 bonding deterministic [\#237](https://github.com/CosmWasm/cw-plus/pull/237) ([maurolacy](https://github.com/maurolacy))
- Upgrade to 0.14.0 alpha2 [\#235](https://github.com/CosmWasm/cw-plus/pull/235) ([maurolacy](https://github.com/maurolacy))
- cw3-fixed-multisig: write cw20 multi-contract mint test [\#223](https://github.com/CosmWasm/cw-plus/pull/223) ([orkunkl](https://github.com/orkunkl))
- Document using tarpaulin [\#222](https://github.com/CosmWasm/cw-plus/pull/222) ([ethanfrey](https://github.com/ethanfrey))
- Juggernaut/add cw20 support [\#221](https://github.com/CosmWasm/cw-plus/pull/221) ([juggernaut09](https://github.com/juggernaut09))
- Multi index generic key [\#211](https://github.com/CosmWasm/cw-plus/pull/211) ([maurolacy](https://github.com/maurolacy))

## [v0.5.0](https://github.com/CosmWasm/cw-plus/tree/v0.5.0) (2021-01-19)

[Full Changelog](https://github.com/CosmWasm/cw-plus/compare/v0.4.0...v0.5.0)

**Closed issues:**

- Fix const\_item\_mutation warnings [\#217](https://github.com/CosmWasm/cw-plus/issues/217)
- Add Prefixer sub-prefixes support [\#214](https://github.com/CosmWasm/cw-plus/issues/214)
- Support composite keys on secondary indexes \(unique-index\) [\#209](https://github.com/CosmWasm/cw-plus/issues/209)
- Update README, helpers [\#208](https://github.com/CosmWasm/cw-plus/issues/208)
- Add \(T, U, V\) triple primary key [\#197](https://github.com/CosmWasm/cw-plus/issues/197)

**Merged pull requests:**

- Update contracts and packages to cw 0.13.2 [\#220](https://github.com/CosmWasm/cw-plus/pull/220) ([orkunkl](https://github.com/orkunkl))
- Payment helpers [\#219](https://github.com/CosmWasm/cw-plus/pull/219) ([ethanfrey](https://github.com/ethanfrey))
- Make self constant in Item::update [\#218](https://github.com/CosmWasm/cw-plus/pull/218) ([webmaster128](https://github.com/webmaster128))
- Prefixer sub prefix [\#215](https://github.com/CosmWasm/cw-plus/pull/215) ([maurolacy](https://github.com/maurolacy))
- Triple primary key 2 [\#213](https://github.com/CosmWasm/cw-plus/pull/213) ([maurolacy](https://github.com/maurolacy))
- Update contract refs to v0.4.0 [\#212](https://github.com/CosmWasm/cw-plus/pull/212) ([maurolacy](https://github.com/maurolacy))
- Implement PrimaryKey for generic \(T, U, V\) triplet [\#210](https://github.com/CosmWasm/cw-plus/pull/210) ([maurolacy](https://github.com/maurolacy))
- Generalize UniqueIndex keys [\#207](https://github.com/CosmWasm/cw-plus/pull/207) ([maurolacy](https://github.com/maurolacy))

## [v0.4.0](https://github.com/CosmWasm/cw-plus/tree/v0.4.0) (2020-12-22)

[Full Changelog](https://github.com/CosmWasm/cw-plus/compare/v0.3.2...v0.4.0)

**Closed issues:**

- Check SnapshotMap with multiple updated [\#187](https://github.com/CosmWasm/cw-plus/issues/187)
- Update to CosmWasm 0.12.2 [\#183](https://github.com/CosmWasm/cw-plus/issues/183)
- Updates to cw3 spec [\#182](https://github.com/CosmWasm/cw-plus/issues/182)
- Pull out admin helper/controller into cw0 [\#181](https://github.com/CosmWasm/cw-plus/issues/181)
- Add flag for errors backtraces [\#178](https://github.com/CosmWasm/cw-plus/issues/178)
- Registered Hooks \(cw0\) should have a "kind" [\#176](https://github.com/CosmWasm/cw-plus/issues/176)
- Add sensible events/attributes to cw3/cw4 handle functions [\#174](https://github.com/CosmWasm/cw-plus/issues/174)
- Update namespaces / constructors to accept &str [\#170](https://github.com/CosmWasm/cw-plus/issues/170)
- Don't use hooks for snapshotting on cw3-cw4 interface [\#162](https://github.com/CosmWasm/cw-plus/issues/162)
- Refactor snapshotting into reusable module [\#161](https://github.com/CosmWasm/cw-plus/issues/161)
- Distinguish between weight 0 and not member in cw3 queries [\#154](https://github.com/CosmWasm/cw-plus/issues/154)
- Migrate strorage-plus to v0.12.0 [\#149](https://github.com/CosmWasm/cw-plus/issues/149)
- Asymmetries between query and execute in CW1 \(subkeys\) [\#145](https://github.com/CosmWasm/cw-plus/issues/145)
- Add token-weighted group [\#142](https://github.com/CosmWasm/cw-plus/issues/142)
- Multisig handles changes to group membership [\#141](https://github.com/CosmWasm/cw-plus/issues/141)
- Add listeners to cw4-group \(and cw4?\) [\#140](https://github.com/CosmWasm/cw-plus/issues/140)
- Update helper.ts files to 0.2.0 [\#66](https://github.com/CosmWasm/cw-plus/issues/66)
- Build bonding curve with cw20-base [\#5](https://github.com/CosmWasm/cw-plus/issues/5)
- Extend threshold types for multisig [\#139](https://github.com/CosmWasm/cw-plus/issues/139)
- Update all contracts to 0.12.0 [\#133](https://github.com/CosmWasm/cw-plus/issues/133)
- Define message callback as cw spec [\#98](https://github.com/CosmWasm/cw-plus/issues/98)
- Separate Groups from Multisigs [\#80](https://github.com/CosmWasm/cw-plus/issues/80)
- Test harness for testing composition [\#9](https://github.com/CosmWasm/cw-plus/issues/9)

**Merged pull requests:**

- Set events for cw4 [\#206](https://github.com/CosmWasm/cw-plus/pull/206) ([ethanfrey](https://github.com/ethanfrey))
- Keep controllers' model private [\#204](https://github.com/CosmWasm/cw-plus/pull/204) ([ethanfrey](https://github.com/ethanfrey))
- Fix cw1-subkeys helper.ts and point to heldernet [\#202](https://github.com/CosmWasm/cw-plus/pull/202) ([orkunkl](https://github.com/orkunkl))
- Fix cw20-base helpers.ts and point to heldernet [\#201](https://github.com/CosmWasm/cw-plus/pull/201) ([orkunkl](https://github.com/orkunkl))
- Claims controller [\#200](https://github.com/CosmWasm/cw-plus/pull/200) ([ethanfrey](https://github.com/ethanfrey))
- Hooks controller [\#195](https://github.com/CosmWasm/cw-plus/pull/195) ([ethanfrey](https://github.com/ethanfrey))
- Create Admin controller [\#194](https://github.com/CosmWasm/cw-plus/pull/194) ([ethanfrey](https://github.com/ethanfrey))
- SnapshotMap properly tracks keys with multiple updates in one block [\#189](https://github.com/CosmWasm/cw-plus/pull/189) ([ethanfrey](https://github.com/ethanfrey))
- Update cw3 spec [\#188](https://github.com/CosmWasm/cw-plus/pull/188) ([ethanfrey](https://github.com/ethanfrey))
- Fix minor errors [\#186](https://github.com/CosmWasm/cw-plus/pull/186) ([ethanfrey](https://github.com/ethanfrey))
- Cw20 bonding curve [\#185](https://github.com/CosmWasm/cw-plus/pull/185) ([ethanfrey](https://github.com/ethanfrey))
- Update all dependencies to 0.12.2 [\#184](https://github.com/CosmWasm/cw-plus/pull/184) ([ethanfrey](https://github.com/ethanfrey))
- Add threshold to cw3 flex [\#180](https://github.com/CosmWasm/cw-plus/pull/180) ([ethanfrey](https://github.com/ethanfrey))
- Replace byte slices by string slices in names and constructors [\#173](https://github.com/CosmWasm/cw-plus/pull/173) ([maurolacy](https://github.com/maurolacy))
- Fix namespace macro test [\#169](https://github.com/CosmWasm/cw-plus/pull/169) ([maurolacy](https://github.com/maurolacy))
- Token weighted group [\#167](https://github.com/CosmWasm/cw-plus/pull/167) ([ethanfrey](https://github.com/ethanfrey))
- Snapshot cw4 \(take 2\) [\#166](https://github.com/CosmWasm/cw-plus/pull/166) ([ethanfrey](https://github.com/ethanfrey))
- Snapshot module [\#164](https://github.com/CosmWasm/cw-plus/pull/164) ([ethanfrey](https://github.com/ethanfrey))
- cw3-flex-multisig uses voting power from a snapshot of the block the proposal opened [\#160](https://github.com/CosmWasm/cw-plus/pull/160) ([ethanfrey](https://github.com/ethanfrey))
- Weight 0 vs not member [\#159](https://github.com/CosmWasm/cw-plus/pull/159) ([maurolacy](https://github.com/maurolacy))
- Weight 0 vs not member [\#158](https://github.com/CosmWasm/cw-plus/pull/158) ([maurolacy](https://github.com/maurolacy))
- Close proposal on membership change [\#157](https://github.com/CosmWasm/cw-plus/pull/157) ([ethanfrey](https://github.com/ethanfrey))
- Add cw4 hooks [\#156](https://github.com/CosmWasm/cw-plus/pull/156) ([ethanfrey](https://github.com/ethanfrey))
- Random Cleanup [\#155](https://github.com/CosmWasm/cw-plus/pull/155) ([ethanfrey](https://github.com/ethanfrey))
- Update cosmwasm version to 0.12.0 [\#148](https://github.com/CosmWasm/cw-plus/pull/148) ([maurolacy](https://github.com/maurolacy))
- Rename CanSend to CanExecute for generality [\#146](https://github.com/CosmWasm/cw-plus/pull/146) ([maurolacy](https://github.com/maurolacy))
- Rename Router -\> App [\#144](https://github.com/CosmWasm/cw-plus/pull/144) ([ethanfrey](https://github.com/ethanfrey))
- Multi test example [\#137](https://github.com/CosmWasm/cw-plus/pull/137) ([ethanfrey](https://github.com/ethanfrey))
- Update to cosmwasm 0.12.0-alpha2 [\#136](https://github.com/CosmWasm/cw-plus/pull/136) ([ethanfrey](https://github.com/ethanfrey))
- Router with rollbacks [\#134](https://github.com/CosmWasm/cw-plus/pull/134) ([ethanfrey](https://github.com/ethanfrey))
- Initial version of helper.ts for CW721-base [\#131](https://github.com/CosmWasm/cw-plus/pull/131) ([volkrass](https://github.com/volkrass))
- Document contract callback technique [\#152](https://github.com/CosmWasm/cw-plus/pull/152) ([ethanfrey](https://github.com/ethanfrey))
- Separate multisig from group [\#150](https://github.com/CosmWasm/cw-plus/pull/150) ([ethanfrey](https://github.com/ethanfrey))
- Sketch integration test framework [\#130](https://github.com/CosmWasm/cw-plus/pull/130) ([ethanfrey](https://github.com/ethanfrey))

## [v0.3.2](https://github.com/CosmWasm/cw-plus/tree/v0.3.2) (2020-10-28)

[Full Changelog](https://github.com/CosmWasm/cw-plus/compare/v0.3.1...v0.3.2)

**Merged pull requests:**

- Fix SendNft in Cw721-base [\#132](https://github.com/CosmWasm/cw-plus/pull/132) ([ethanfrey](https://github.com/ethanfrey))
- Define groups [\#129](https://github.com/CosmWasm/cw-plus/pull/129) ([ethanfrey](https://github.com/ethanfrey))

## [v0.3.1](https://github.com/CosmWasm/cw-plus/tree/v0.3.1) (2020-10-16)

[Full Changelog](https://github.com/CosmWasm/cw-plus/compare/v0.3.0...v0.3.1)

**Closed issues:**

- Update to CosmWasm 0.11.1 [\#127](https://github.com/CosmWasm/cw-plus/issues/127)
- Fix compiler warning \(const\_item\_mutation\) [\#123](https://github.com/CosmWasm/cw-plus/issues/123)
- Implement TokensByOwner on base NFT contract [\#81](https://github.com/CosmWasm/cw-plus/issues/81)

**Merged pull requests:**

- Bump cosmwasm version [\#128](https://github.com/CosmWasm/cw-plus/pull/128) ([ethanfrey](https://github.com/ethanfrey))
- OwnedBound -\> Option\<Bound\> [\#126](https://github.com/CosmWasm/cw-plus/pull/126) ([ethanfrey](https://github.com/ethanfrey))
- Static index type [\#125](https://github.com/CosmWasm/cw-plus/pull/125) ([ethanfrey](https://github.com/ethanfrey))
- Update Rust compiler [\#124](https://github.com/CosmWasm/cw-plus/pull/124) ([webmaster128](https://github.com/webmaster128))
- Add TokensByOwner for cw721-base [\#122](https://github.com/CosmWasm/cw-plus/pull/122) ([ethanfrey](https://github.com/ethanfrey))
- Secondary indexes [\#108](https://github.com/CosmWasm/cw-plus/pull/108) ([ethanfrey](https://github.com/ethanfrey))

## [v0.3.0](https://github.com/CosmWasm/cw-plus/tree/v0.3.0) (2020-10-12)

[Full Changelog](https://github.com/CosmWasm/cw-plus/compare/v0.2.3...v0.3.0)

**Closed issues:**

- Building contracts failed [\#117](https://github.com/CosmWasm/cw-plus/issues/117)
- Remove dependency between cw20\_escrow and cw20\_atomic\_swap [\#115](https://github.com/CosmWasm/cw-plus/issues/115)
- Fix Claims handling in cw20-staking [\#110](https://github.com/CosmWasm/cw-plus/issues/110)
- Migrate contracts to v0.11 [\#96](https://github.com/CosmWasm/cw-plus/issues/96)

**Merged pull requests:**

- Fix workspace optimizer [\#121](https://github.com/CosmWasm/cw-plus/pull/121) ([ethanfrey](https://github.com/ethanfrey))
- Migrate cw3-fixed-multisig [\#119](https://github.com/CosmWasm/cw-plus/pull/119) ([ethanfrey](https://github.com/ethanfrey))
- Move shared Balance struct to cw20 [\#118](https://github.com/CosmWasm/cw-plus/pull/118) ([maurolacy](https://github.com/maurolacy))
- Use Include/Exclude Bounds to define range searches [\#116](https://github.com/CosmWasm/cw-plus/pull/116) ([ethanfrey](https://github.com/ethanfrey))
- Merge 0.2.x into master [\#114](https://github.com/CosmWasm/cw-plus/pull/114) ([ethanfrey](https://github.com/ethanfrey))
- Migrate to v0.11.0 [\#113](https://github.com/CosmWasm/cw-plus/pull/113) ([ethanfrey](https://github.com/ethanfrey))
- Finish v0.11 migration [\#111](https://github.com/CosmWasm/cw-plus/pull/111) ([ethanfrey](https://github.com/ethanfrey))
- Use Maps for storage [\#109](https://github.com/CosmWasm/cw-plus/pull/109) ([ethanfrey](https://github.com/ethanfrey))
- Migrate to v0.11 2 [\#107](https://github.com/CosmWasm/cw-plus/pull/107) ([maurolacy](https://github.com/maurolacy))
- Migrate to v0.11 [\#104](https://github.com/CosmWasm/cw-plus/pull/104) ([maurolacy](https://github.com/maurolacy))

## [v0.2.3](https://github.com/CosmWasm/cw-plus/tree/v0.2.3) (2020-10-10)

[Full Changelog](https://github.com/CosmWasm/cw-plus/compare/v0.2.2...v0.2.3)

**Closed issues:**

- Migration to 0.11: errors of shared functions accross contracts [\#103](https://github.com/CosmWasm/cw-plus/issues/103)
- Look at serde\(flatten\) to simplify return value composition [\#57](https://github.com/CosmWasm/cw-plus/issues/57)

**Merged pull requests:**

- Better staking claims [\#112](https://github.com/CosmWasm/cw-plus/pull/112) ([ethanfrey](https://github.com/ethanfrey))

## [v0.2.2](https://github.com/CosmWasm/cw-plus/tree/v0.2.2) (2020-09-30)

[Full Changelog](https://github.com/CosmWasm/cw-plus/compare/v0.2.1...v0.2.2)

**Closed issues:**

- calc\_range\_start to cw0 [\#101](https://github.com/CosmWasm/cw-plus/issues/101)
- Avoid sending zero amount cw20 tokens [\#89](https://github.com/CosmWasm/cw-plus/issues/89)
- Unify handling of native and cw20 coins in contracts [\#88](https://github.com/CosmWasm/cw-plus/issues/88)
- Define cw3 spec for multisigs [\#79](https://github.com/CosmWasm/cw-plus/issues/79)
- Check for / reject zero amount tokens [\#76](https://github.com/CosmWasm/cw-plus/issues/76)
- Implement base NFT contract [\#44](https://github.com/CosmWasm/cw-plus/issues/44)
- Basic multisig contract [\#8](https://github.com/CosmWasm/cw-plus/issues/8)

**Merged pull requests:**

- Fix calc range [\#102](https://github.com/CosmWasm/cw-plus/pull/102) ([ethanfrey](https://github.com/ethanfrey))
- Fix CLI call command [\#100](https://github.com/CosmWasm/cw-plus/pull/100) ([webmaster128](https://github.com/webmaster128))
- Implement cw721-base nft contract [\#97](https://github.com/CosmWasm/cw-plus/pull/97) ([ethanfrey](https://github.com/ethanfrey))
- Unit tests for cw3-fixed-multisig [\#95](https://github.com/CosmWasm/cw-plus/pull/95) ([maurolacy](https://github.com/maurolacy))
- Add zero amount checks / tests [\#94](https://github.com/CosmWasm/cw-plus/pull/94) ([maurolacy](https://github.com/maurolacy))
- cw20-escrow refactoring: Unify handling of native and cw20 [\#92](https://github.com/CosmWasm/cw-plus/pull/92) ([maurolacy](https://github.com/maurolacy))
- Cw3 fixed multisig [\#91](https://github.com/CosmWasm/cw-plus/pull/91) ([ethanfrey](https://github.com/ethanfrey))
- Cw3 spec [\#90](https://github.com/CosmWasm/cw-plus/pull/90) ([ethanfrey](https://github.com/ethanfrey))
- Native balance refactoring [\#87](https://github.com/CosmWasm/cw-plus/pull/87) ([maurolacy](https://github.com/maurolacy))
- Cw20 zero amount checks [\#86](https://github.com/CosmWasm/cw-plus/pull/86) ([maurolacy](https://github.com/maurolacy))
- Update helpers source tags and builder info [\#85](https://github.com/CosmWasm/cw-plus/pull/85) ([orkunkl](https://github.com/orkunkl))

## [v0.2.1](https://github.com/CosmWasm/cw-plus/tree/v0.2.1) (2020-09-10)

[Full Changelog](https://github.com/CosmWasm/cw-plus/compare/v0.2.0...v0.2.1)

**Closed issues:**

- Implement Copy for Coin / Vec\<Coin\> [\#77](https://github.com/CosmWasm/cw-plus/issues/77)
- Why does not cw20 pass the received native token? [\#74](https://github.com/CosmWasm/cw-plus/issues/74)
- Cw20Coin duplication [\#73](https://github.com/CosmWasm/cw-plus/issues/73)
- Fix docker run script in all contract README [\#69](https://github.com/CosmWasm/cw-plus/issues/69)
- Add cw20 support to atomic swap contract [\#27](https://github.com/CosmWasm/cw-plus/issues/27)
- Implement atomic swap contract with native tokens [\#26](https://github.com/CosmWasm/cw-plus/issues/26)

**Merged pull requests:**

- Update workspace optimizer version to 0.10.3 [\#83](https://github.com/CosmWasm/cw-plus/pull/83) ([orkunkl](https://github.com/orkunkl))
- cw1-subkeys: Point helper to smart contract version v0.2.1 [\#82](https://github.com/CosmWasm/cw-plus/pull/82) ([orkunkl](https://github.com/orkunkl))
- Cw20coin refactoring [\#78](https://github.com/CosmWasm/cw-plus/pull/78) ([maurolacy](https://github.com/maurolacy))
- cw1-subkeys: Implement permission functionality [\#75](https://github.com/CosmWasm/cw-plus/pull/75) ([orkunkl](https://github.com/orkunkl))
- Cw20 atomic swaps [\#72](https://github.com/CosmWasm/cw-plus/pull/72) ([maurolacy](https://github.com/maurolacy))
- Update contracts README \(workspace-optimizer\) [\#71](https://github.com/CosmWasm/cw-plus/pull/71) ([maurolacy](https://github.com/maurolacy))
- Update with new wasm, new queries [\#70](https://github.com/CosmWasm/cw-plus/pull/70) ([ethanfrey](https://github.com/ethanfrey))
- Subkeys details [\#68](https://github.com/CosmWasm/cw-plus/pull/68) ([maurolacy](https://github.com/maurolacy))
- Atomic swap [\#52](https://github.com/CosmWasm/cw-plus/pull/52) ([maurolacy](https://github.com/maurolacy))

## [v0.2.0](https://github.com/CosmWasm/cw-plus/tree/v0.2.0) (2020-08-28)

[Full Changelog](https://github.com/CosmWasm/cw-plus/compare/workshop-pre-cw20-staking...v0.2.0)

**Closed issues:**

- Support contract migration [\#63](https://github.com/CosmWasm/cw-plus/issues/63)
- Add way to list allowances on an account to cw20 [\#55](https://github.com/CosmWasm/cw-plus/issues/55)
- Add "ListAccounts" to cw20-base [\#48](https://github.com/CosmWasm/cw-plus/issues/48)
- Add README in project root [\#47](https://github.com/CosmWasm/cw-plus/issues/47)

**Merged pull requests:**

- Cw20-base migration [\#67](https://github.com/CosmWasm/cw-plus/pull/67) ([ethanfrey](https://github.com/ethanfrey))
- Add readme [\#65](https://github.com/CosmWasm/cw-plus/pull/65) ([ethanfrey](https://github.com/ethanfrey))
- Cw20 base iterators [\#64](https://github.com/CosmWasm/cw-plus/pull/64) ([ethanfrey](https://github.com/ethanfrey))
- workshop subkey PR [\#62](https://github.com/CosmWasm/cw-plus/pull/62) ([whalelephant](https://github.com/whalelephant))
- Add cw20 functionality to the staking contract [\#60](https://github.com/CosmWasm/cw-plus/pull/60) ([ethanfrey](https://github.com/ethanfrey))
- Add basic staking derivatives as CW20 token contracts [\#58](https://github.com/CosmWasm/cw-plus/pull/58) ([ethanfrey](https://github.com/ethanfrey))

## [workshop-pre-cw20-staking](https://github.com/CosmWasm/cw-plus/tree/workshop-pre-cw20-staking) (2020-08-26)

[Full Changelog](https://github.com/CosmWasm/cw-plus/compare/v0.1.1...workshop-pre-cw20-staking)

**Closed issues:**

- Define Spec of NFTs [\#43](https://github.com/CosmWasm/cw-plus/issues/43)
- Release and Publish v0.1.1 [\#40](https://github.com/CosmWasm/cw-plus/issues/40)
- Add @cosmjs/cli helpers for each contract [\#38](https://github.com/CosmWasm/cw-plus/issues/38)

**Merged pull requests:**

- Bump all CosmWasm dependencies to 0.10.1 [\#56](https://github.com/CosmWasm/cw-plus/pull/56) ([ethanfrey](https://github.com/ethanfrey))
- Add new query to return all allowances on subkeys [\#54](https://github.com/CosmWasm/cw-plus/pull/54) ([ethanfrey](https://github.com/ethanfrey))
- Add CanSend query to the cw1 spec [\#53](https://github.com/CosmWasm/cw-plus/pull/53) ([ethanfrey](https://github.com/ethanfrey))
- Add Expration to cw0 [\#51](https://github.com/CosmWasm/cw-plus/pull/51) ([ethanfrey](https://github.com/ethanfrey))
- Nft 721 spec [\#50](https://github.com/CosmWasm/cw-plus/pull/50) ([ethanfrey](https://github.com/ethanfrey))
- Add Subkeys helper [\#49](https://github.com/CosmWasm/cw-plus/pull/49) ([ethanfrey](https://github.com/ethanfrey))
- Add helpers to cw20-base [\#46](https://github.com/CosmWasm/cw-plus/pull/46) ([ethanfrey](https://github.com/ethanfrey))

## [v0.1.1](https://github.com/CosmWasm/cw-plus/tree/v0.1.1) (2020-08-13)

[Full Changelog](https://github.com/CosmWasm/cw-plus/compare/v0.1.0...v0.1.1)

**Closed issues:**

- Implement cw2-migrate for all contracts [\#35](https://github.com/CosmWasm/cw-plus/issues/35)

## [v0.1.0](https://github.com/CosmWasm/cw-plus/tree/v0.1.0) (2020-08-13)

[Full Changelog](https://github.com/CosmWasm/cw-plus/compare/3f5495857182e183e5832dc3a7913d8ed751659e...v0.1.0)

**Closed issues:**

- Use more unique names for store/queries \(not "Config" / "Meta"\) [\#37](https://github.com/CosmWasm/cw-plus/issues/37)
- Convert all existing code to apache license [\#36](https://github.com/CosmWasm/cw-plus/issues/36)
- Upgrade contracts to 0.10 [\#31](https://github.com/CosmWasm/cw-plus/issues/31)
- Avoid linking contract into contract [\#30](https://github.com/CosmWasm/cw-plus/issues/30)
- Fix DoS attack on cw20-escrow [\#19](https://github.com/CosmWasm/cw-plus/issues/19)
- Implement allowances on cw20-base contract [\#15](https://github.com/CosmWasm/cw-plus/issues/15)
- Implement mintable for cw20-base contract [\#13](https://github.com/CosmWasm/cw-plus/issues/13)
- Build cw20-escrow contract [\#6](https://github.com/CosmWasm/cw-plus/issues/6)
- Set up CI [\#4](https://github.com/CosmWasm/cw-plus/issues/4)
- Define CW20-base [\#3](https://github.com/CosmWasm/cw-plus/issues/3)
- Define CW20 spec [\#2](https://github.com/CosmWasm/cw-plus/issues/2)
- Add CLA bot [\#1](https://github.com/CosmWasm/cw-plus/issues/1)

**Merged pull requests:**

- Add migration info to contracts [\#45](https://github.com/CosmWasm/cw-plus/pull/45) ([ethanfrey](https://github.com/ethanfrey))
- Add optimization config to all contracts in Cargo.toml [\#42](https://github.com/CosmWasm/cw-plus/pull/42) ([ethanfrey](https://github.com/ethanfrey))
- Agpl to apache [\#41](https://github.com/CosmWasm/cw-plus/pull/41) ([ethanfrey](https://github.com/ethanfrey))
- Unique singleton names [\#39](https://github.com/CosmWasm/cw-plus/pull/39) ([ethanfrey](https://github.com/ethanfrey))
- Cw2 migrate spec [\#34](https://github.com/CosmWasm/cw-plus/pull/34) ([ethanfrey](https://github.com/ethanfrey))
- Update to 0.10.0 final [\#33](https://github.com/CosmWasm/cw-plus/pull/33) ([maurolacy](https://github.com/maurolacy))
- Enable contracts to import contracts [\#32](https://github.com/CosmWasm/cw-plus/pull/32) ([ethanfrey](https://github.com/ethanfrey))
- Add deployment job to CI [\#29](https://github.com/CosmWasm/cw-plus/pull/29) ([webmaster128](https://github.com/webmaster128))
- Subkeys 2 [\#28](https://github.com/CosmWasm/cw-plus/pull/28) ([maurolacy](https://github.com/maurolacy))
- Update to 0.10 [\#25](https://github.com/CosmWasm/cw-plus/pull/25) ([maurolacy](https://github.com/maurolacy))
- Implement subkeys as a cw1 contract [\#24](https://github.com/CosmWasm/cw-plus/pull/24) ([ethanfrey](https://github.com/ethanfrey))
- Rename multisig to whitelist [\#23](https://github.com/CosmWasm/cw-plus/pull/23) ([ethanfrey](https://github.com/ethanfrey))
- Add Cw1 for proxy contracts [\#22](https://github.com/CosmWasm/cw-plus/pull/22) ([ethanfrey](https://github.com/ethanfrey))
- Cw20 allowances [\#21](https://github.com/CosmWasm/cw-plus/pull/21) ([ethanfrey](https://github.com/ethanfrey))
- Fix escrow DoS Attack [\#20](https://github.com/CosmWasm/cw-plus/pull/20) ([ethanfrey](https://github.com/ethanfrey))
- Cw20 base mintable [\#18](https://github.com/CosmWasm/cw-plus/pull/18) ([ethanfrey](https://github.com/ethanfrey))
- Cw20 escrow [\#16](https://github.com/CosmWasm/cw-plus/pull/16) ([ethanfrey](https://github.com/ethanfrey))
- Cleanup contract [\#14](https://github.com/CosmWasm/cw-plus/pull/14) ([ethanfrey](https://github.com/ethanfrey))
- Create basic Cw20 contract \(reference\) [\#12](https://github.com/CosmWasm/cw-plus/pull/12) ([ethanfrey](https://github.com/ethanfrey))
- Define all Message and Query types [\#11](https://github.com/CosmWasm/cw-plus/pull/11) ([ethanfrey](https://github.com/ethanfrey))
- Set up basic CI script [\#10](https://github.com/CosmWasm/cw-plus/pull/10) ([ethanfrey](https://github.com/ethanfrey))



\* *This Changelog was automatically generated by [github_changelog_generator](https://github.com/github-changelog-generator/github-changelog-generator)*
