// Copyright 2019 ChainSafe Systems (ON) Corp.
// This file is part of gossamer.
//
// The gossamer library is free software: you can redistribute it and/or modify
// it under the terms of the GNU Lesser General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// The gossamer library is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Lesser General Public License for more details.
//
// You should have received a copy of the GNU Lesser General Public License
// along with the gossamer library. If not, see <http://www.gnu.org/licenses/>.

package runtime

import (
	"github.com/ChainSafe/gossamer/lib/common"
)

//nolint
const (
	SUBSTRATE_TEST_RUNTIME     = "substrate_test_runtime"
	SUBSTRATE_TEST_RUNTIME_FP  = "substrate_test_runtime.compact.wasm"
	SUBSTRATE_TEST_RUNTIME_URL = "https://github.com/noot/substrate/blob/add-blob-042920/target/wasm32-unknown-unknown/release/wbuild/substrate-test-runtime/substrate_test_runtime.compact.wasm?raw=true"

	// v0.6 substrate runtime
	LEGACY_NODE_RUNTIME     = "legacy_node_runtime"
	LEGACY_NODE_RUNTIME_FP  = "legacy_node_runtime.compact.wasm"
	LEGACY_NODE_RUNTIME_URL = "https://github.com/noot/substrate/blob/noot/legacy/target/wasm32-unknown-unknown/release/wbuild/node-runtime/node_runtime.compact.wasm?raw=true"

	// v0.8 substrate runtime
	NODE_RUNTIME     = "node_runtime"
	NODE_RUNTIME_FP  = "node_runtime.compact.wasm"
	NODE_RUNTIME_URL = "https://github.com/noot/substrate/blob/noot/v0.8/target/wasm32-unknown-unknown/release/wbuild/node-runtime/node_runtime.compact.wasm?raw=true"

	TEST_RUNTIME  = "test_runtime"
	TESTS_FP      = "test_wasm.wasm"
	TEST_WASM_URL = "https://github.com/ChainSafe/gossamer-test-wasm/blob/noot/target/wasm32-unknown-unknown/release/test_wasm.wasm?raw=true"
)

var (
	// CoreVersion is the runtime API call Core_version
	CoreVersion = "Core_version"
	// CoreInitializeBlock is the runtime API call Core_initialize_block
	CoreInitializeBlock = "Core_initialize_block"
	// CoreExecuteBlock is the runtime API call Core_execute_block
	CoreExecuteBlock = "Core_execute_block"
	// Metadata is the runtime API call Metadata_metadata
	Metadata = "Metadata_metadata"
	// TaggedTransactionQueueValidateTransaction is the runtime API call TaggedTransactionQueue_validate_transaction
	TaggedTransactionQueueValidateTransaction = "TaggedTransactionQueue_validate_transaction"
	// GrandpaAuthorities is the runtime API call GrandpaApi_grandpa_authorities
	GrandpaAuthorities = "GrandpaApi_grandpa_authorities"
	// BabeAPIConfiguration is the runtime API call BabeApi_configuration
	BabeAPIConfiguration = "BabeApi_configuration"
	// BlockBuilderInherentExtrinsics is the runtime API call BlockBuilder_inherent_extrinsics
	BlockBuilderInherentExtrinsics = "BlockBuilder_inherent_extrinsics"
	// BlockBuilderApplyExtrinsic is the runtime API call BlockBuilder_apply_extrinsic
	BlockBuilderApplyExtrinsic = "BlockBuilder_apply_extrinsic"
	// BlockBuilderFinalizeBlock is the runtime API call BlockBuilder_finalize_block
	BlockBuilderFinalizeBlock = "BlockBuilder_finalize_block"
)

// GrandpaAuthorityDataKey is the location of GRANDPA authority data in the storage trie for LEGACY_NODE_RUNTIME and NODE_RUNTIME
var GrandpaAuthorityDataKey, _ = common.HexToBytes("0x3a6772616e6470615f617574686f726974696573")

// BABEPrefix is the prefix for all BABE related storage values
var BABEPrefix, _ = common.Twox128Hash([]byte("Babe"))

// BABEAuthorityDataKey is the location of the BABE authorities in the storage trie for NODE_RUNTIME
func BABEAuthorityDataKey() []byte {
	key, _ := common.Twox128Hash([]byte("Authorities"))
	return append(BABEPrefix, key...)
}

// BABERandomnessKey is the location of the BABE initial randomness in the storage trie for NODE_RUNTIME
func BABERandomnessKey() []byte {
	key, _ := common.Twox128Hash([]byte("Randomness"))
	return append(BABEPrefix, key...)
}
