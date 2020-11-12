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

package wasmer

// #include <stdlib.h>
//
// extern void ext_logging_log_version_1(void *context, int32_t level, int64_t target, int64_t msg);
//
// extern void ext_sandbox_instance_teardown_version_1(void *context, int32_t a);
// extern int32_t ext_sandbox_instantiate_version_1(void *context, int32_t a, int64_t b, int64_t c, int32_t d);
// extern int32_t ext_sandbox_invoke_version_1(void *context, int32_t a, int64_t b, int64_t c, int32_t d, int32_t e, int32_t f);
// extern int32_t ext_sandbox_memory_get_version_1(void *context, int32_t a, int32_t b, int32_t c, int32_t d);
// extern int32_t ext_sandbox_memory_new_version_1(void *context, int32_t a, int32_t b);
// extern int32_t ext_sandbox_memory_set_version_1(void *context, int32_t a, int32_t b, int32_t c, int32_t d);
// extern void ext_sandbox_memory_teardown_version_1(void *context, int32_t a);
//
// extern int32_t ext_crypto_ed25519_generate_version_1(void *context, int32_t a, int64_t b);
// extern int64_t ext_crypto_ed25519_public_keys_version_1(void *context, int32_t a);
// extern int64_t ext_crypto_ed25519_sign_version_1(void *context, int32_t a, int32_t b, int64_t c);
// extern int32_t ext_crypto_ed25519_verify_version_1(void *context, int32_t a, int64_t b, int32_t c);
// extern int32_t ext_crypto_finish_batch_verify_version_1(void *context);
// extern int64_t ext_crypto_secp256k1_ecdsa_recover_version_1(void *context, int32_t a, int32_t b);
// extern int64_t ext_crypto_secp256k1_ecdsa_recover_compressed_version_1(void *context, int32_t a, int32_t b);
// extern int32_t ext_crypto_sr25519_generate_version_1(void *context, int32_t a, int64_t b);
// extern int64_t ext_crypto_sr25519_public_keys_version_1(void *context, int32_t a);
// extern int64_t ext_crypto_sr25519_sign_version_1(void *context, int32_t a, int32_t b, int64_t c);
// extern int32_t ext_crypto_sr25519_verify_version_1(void *context, int32_t a, int64_t b, int32_t c);
// extern int32_t ext_crypto_sr25519_verify_version_2(void *context, int32_t a, int64_t b, int32_t c);
// extern void ext_crypto_start_batch_verify_version_1(void *context);
//
// extern int32_t ext_trie_blake2_256_root_version_1(void *context, int64_t a);
// extern int32_t ext_trie_blake2_256_ordered_root_version_1(void *context, int64_t a);
//
// extern void ext_misc_print_hex_version_1(void *context, int64_t a);
// extern void ext_misc_print_num_version_1(void *context, int64_t a);
// extern void ext_misc_print_utf8_version_1(void *context, int64_t a);
// extern int64_t ext_misc_runtime_version_version_1(void *context, int64_t a);
//
// extern void ext_default_child_storage_clear_version_1(void *context, int64_t a, int64_t b);
// extern int64_t ext_default_child_storage_get_version_1(void *context, int64_t a, int64_t b);
// extern int64_t ext_default_child_storage_next_key_version_1(void *context, int64_t a, int64_t b);
// extern int64_t ext_default_child_storage_read_version_1(void *context, int64_t a, int64_t b, int64_t c, int32_t d);
// extern int64_t ext_default_child_storage_root_version_1(void *context, int64_t a);
// extern void ext_default_child_storage_set_version_1(void *context, int64_t a, int64_t b, int64_t c);
// extern void ext_default_child_storage_storage_kill_version_1(void *context, int64_t a);
// extern void ext_default_child_storage_clear_prefix_version_1(void *context, int64_t a, int64_t b);
// extern int32_t ext_default_child_storage_exists_version_1(void *context, int64_t a, int64_t b);
//
// extern void ext_allocator_free_version_1(void *context, int32_t a);
// extern int32_t ext_allocator_malloc_version_1(void *context, int32_t a);
//
// extern int32_t ext_hashing_blake2_128_version_1(void *context, int64_t a);
// extern int32_t ext_hashing_blake2_256_version_1(void *context, int64_t a);
// extern int32_t ext_hashing_keccak_256_version_1(void *context, int64_t a);
// extern int32_t ext_hashing_sha2_256_version_1(void *context, int64_t a);
// extern int32_t ext_hashing_twox_256_version_1(void *context, int64_t a);
// extern int32_t ext_hashing_twox_128_version_1(void *context, int64_t a);
// extern int32_t ext_hashing_twox_64_version_1(void *context, int64_t a);
//
// extern void ext_offchain_index_set_version_1(void *context, int64_t a, int64_t b);
// extern int32_t ext_offchain_is_validator_version_1(void *context);
// extern int32_t ext_offchain_local_storage_compare_and_set_version_1(void *context, int32_t a, int64_t b, int64_t c, int64_t d);
// extern int64_t ext_offchain_local_storage_get_version_1(void *context, int32_t a, int64_t b);
// extern void ext_offchain_local_storage_set_version_1(void *context, int32_t a, int64_t b, int64_t c);
// extern int64_t ext_offchain_network_state_version_1(void *context);
// extern int32_t ext_offchain_random_seed_version_1(void *context);
// extern int64_t ext_offchain_submit_transaction_version_1(void *context, int64_t a);
//
// extern void ext_storage_append_version_1(void *context, int64_t a, int64_t b);
// extern int64_t ext_storage_changes_root_version_1(void *context, int64_t a);
// extern void ext_storage_clear_version_1(void *context, int64_t a);
// extern void ext_storage_clear_prefix_version_1(void *context, int64_t a);
// extern void ext_storage_commit_transaction_version_1(void *context);
// extern int32_t ext_storage_exists_version_1(void *context, int64_t a);
// extern int64_t ext_storage_get_version_1(void *context, int64_t a);
// extern int64_t ext_storage_next_key_version_1(void *context, int64_t a);
// extern int64_t ext_storage_read_version_1(void *context, int64_t a, int64_t b, int32_t c);
// extern void ext_storage_rollback_transaction_version_1(void *context);
// extern int64_t ext_storage_root_version_1(void *context);
// extern void ext_storage_set_version_1(void *context, int64_t a, int64_t b);
// extern void ext_storage_start_transaction_version_1(void *context);
import "C"

import (
	"errors"
	"fmt"
	"unsafe"

	"github.com/ChainSafe/gossamer/lib/common"
	"github.com/ChainSafe/gossamer/lib/common/optional"
	"github.com/ChainSafe/gossamer/lib/runtime"
	"github.com/ChainSafe/gossamer/lib/trie"

	wasm "github.com/wasmerio/go-ext-wasm/wasmer"
)

//export ext_logging_log_version_1
func ext_logging_log_version_1(context unsafe.Pointer, level C.int32_t, targetData, msgData C.int64_t) {
	logger.Trace("[ext_logging_log_version_1] executing...")
	instanceContext := wasm.IntoInstanceContext(context)
	memory := instanceContext.Memory().Data()

	targetPtr, targetSize := int64ToPointerAndSize(int64(targetData))
	target := fmt.Sprintf("%s", memory[targetPtr:targetPtr+targetSize])
	msgPtr, msgSize := int64ToPointerAndSize(int64(msgData))
	msg := fmt.Sprintf("%s", memory[msgPtr:msgPtr+msgSize])

	switch int(level) {
	case 0:
		logger.Crit("[ext_logging_log_version_1]", "target", target, "message", msg)
	case 1:
		logger.Warn("[ext_logging_log_version_1]", "target", target, "message", msg)
	case 2:
		logger.Info("[ext_logging_log_version_1]", "target", target, "message", msg)
	case 3:
		logger.Debug("[ext_logging_log_version_1]", "target", target, "message", msg)
	case 4:
		logger.Trace("[ext_logging_log_version_1]", "target", target, "message", msg)
	}
}

//export ext_sandbox_instance_teardown_version_1
func ext_sandbox_instance_teardown_version_1(context unsafe.Pointer, a C.int32_t) {
	logger.Trace("[ext_sandbox_instance_teardown_version_1] executing...")
}

//export ext_sandbox_instantiate_version_1
func ext_sandbox_instantiate_version_1(context unsafe.Pointer, a C.int32_t, x, y C.int64_t, z C.int32_t) C.int32_t {
	logger.Trace("[ext_sandbox_instantiate_version_1] executing...")
	return 0
}

//export ext_sandbox_invoke_version_1
func ext_sandbox_invoke_version_1(context unsafe.Pointer, a C.int32_t, x, y C.int64_t, z, d, e C.int32_t) C.int32_t {
	logger.Trace("[ext_sandbox_invoke_version_1] executing...")
	return 0
}

//export ext_sandbox_memory_get_version_1
func ext_sandbox_memory_get_version_1(context unsafe.Pointer, a, z, d, e C.int32_t) C.int32_t {
	logger.Trace("[ext_sandbox_memory_get_version_1] executing...")
	return 0
}

//export ext_sandbox_memory_new_version_1
func ext_sandbox_memory_new_version_1(context unsafe.Pointer, a, z C.int32_t) C.int32_t {
	logger.Trace("[ext_sandbox_memory_new_version_1] executing...")
	return 0
}

//export ext_sandbox_memory_set_version_1
func ext_sandbox_memory_set_version_1(context unsafe.Pointer, a, z, d, e C.int32_t) C.int32_t {
	logger.Trace("[ext_sandbox_memory_set_version_1] executing...")
	return 0
}

//export ext_sandbox_memory_teardown_version_1
func ext_sandbox_memory_teardown_version_1(context unsafe.Pointer, a C.int32_t) {
	logger.Trace("[ext_sandbox_memory_teardown_version_1] executing...")
}

//export ext_crypto_ed25519_generate_version_1
func ext_crypto_ed25519_generate_version_1(context unsafe.Pointer, a C.int32_t, z C.int64_t) C.int32_t {
	logger.Trace("[ext_crypto_ed25519_generate_version_1] executing...")
	return 0
}

//export ext_crypto_ed25519_public_keys_version_1
func ext_crypto_ed25519_public_keys_version_1(context unsafe.Pointer, a C.int32_t) C.int64_t {
	logger.Trace("[ext_crypto_ed25519_public_keys_version_1] executing...")
	return 0
}

//export ext_crypto_ed25519_sign_version_1
func ext_crypto_ed25519_sign_version_1(context unsafe.Pointer, a, z C.int32_t, y C.int64_t) C.int64_t {
	logger.Trace("[ext_crypto_ed25519_sign_version_1] executing...")
	return 0
}

//export ext_crypto_ed25519_verify_version_1
func ext_crypto_ed25519_verify_version_1(context unsafe.Pointer, a C.int32_t, z C.int64_t, y C.int32_t) C.int32_t {
	logger.Trace("[ext_crypto_ed25519_verify_version_1] executing...")
	return 0
}

//export ext_crypto_finish_batch_verify_version_1
func ext_crypto_finish_batch_verify_version_1(context unsafe.Pointer) C.int32_t {
	logger.Trace("[ext_crypto_finish_batch_verify_version_1] executing...")
	return 0
}

//export ext_crypto_secp256k1_ecdsa_recover_version_1
func ext_crypto_secp256k1_ecdsa_recover_version_1(context unsafe.Pointer, a, z C.int32_t) C.int64_t {
	logger.Trace("[ext_crypto_secp256k1_ecdsa_recover_version_1] executing...")
	return 0
}

//export ext_crypto_secp256k1_ecdsa_recover_compressed_version_1
func ext_crypto_secp256k1_ecdsa_recover_compressed_version_1(context unsafe.Pointer, a, z C.int32_t) C.int64_t {
	logger.Trace("[ext_crypto_secp256k1_ecdsa_recover_compressed_version_1] executing...")
	return 0
}

//export ext_crypto_sr25519_generate_version_1
func ext_crypto_sr25519_generate_version_1(context unsafe.Pointer, a C.int32_t, z C.int64_t) C.int32_t {
	logger.Trace("[ext_crypto_sr25519_generate_version_1] executing...")
	return 0
}

//export ext_crypto_sr25519_public_keys_version_1
func ext_crypto_sr25519_public_keys_version_1(context unsafe.Pointer, a C.int32_t) C.int64_t {
	logger.Trace("[ext_crypto_sr25519_public_keys_version_1] executing...")
	return 0
}

//export ext_crypto_sr25519_sign_version_1
func ext_crypto_sr25519_sign_version_1(context unsafe.Pointer, a, z C.int32_t, y C.int64_t) C.int64_t {
	logger.Trace("[ext_crypto_sr25519_sign_version_1] executing...")
	return 0
}

//export ext_crypto_sr25519_verify_version_1
func ext_crypto_sr25519_verify_version_1(context unsafe.Pointer, a C.int32_t, z C.int64_t, y C.int32_t) C.int32_t {
	logger.Trace("[ext_crypto_sr25519_verify_version_1] executing...")
	return 0
}

//export ext_crypto_sr25519_verify_version_2
func ext_crypto_sr25519_verify_version_2(context unsafe.Pointer, a C.int32_t, z C.int64_t, y C.int32_t) C.int32_t {
	logger.Trace("[ext_crypto_sr25519_verify_version_2] executing...")
	return 0
}

//export ext_crypto_start_batch_verify_version_1
func ext_crypto_start_batch_verify_version_1(context unsafe.Pointer) {
	logger.Trace("[ext_crypto_start_batch_verify_version_1] executing...")
}

//export ext_trie_blake2_256_root_version_1
func ext_trie_blake2_256_root_version_1(context unsafe.Pointer, data C.int64_t) C.int32_t {
	logger.Trace("[ext_trie_blake2_256_root_version_1] executing...")
	return 0
}

//export ext_trie_blake2_256_ordered_root_version_1
func ext_trie_blake2_256_ordered_root_version_1(context unsafe.Pointer, data C.int64_t) C.int32_t {
	logger.Trace("[ext_trie_blake2_256_ordered_root_version_1] executing...")
	//dataPtr, dataSize := int64ToPointerAndSize(int64(data))

	instanceContext := wasm.IntoInstanceContext(context)
	memory := instanceContext.Memory().Data()
	runtimeCtx := instanceContext.Data().(*runtime.Context)

	// allocate memory for value and copy value to memory
	ptr, err := runtimeCtx.Allocator.Allocate(32)
	if err != nil {
		logger.Error("[ext_trie_blake2_256_ordered_root_version_1]", "error", err)
		return 0
	}

	copy(memory[ptr:ptr+32], trie.EmptyHash[:])
	return C.int32_t(ptr)
}

//export ext_misc_print_hex_version_1
func ext_misc_print_hex_version_1(context unsafe.Pointer, a C.int64_t) {
	logger.Trace("[ext_misc_print_hex_version_1] executing...")
}

//export ext_misc_print_num_version_1
func ext_misc_print_num_version_1(context unsafe.Pointer, a C.int64_t) {
	logger.Trace("[ext_misc_print_num_version_1] executing...")
}

//export ext_misc_print_utf8_version_1
func ext_misc_print_utf8_version_1(context unsafe.Pointer, data C.int64_t) {
	logger.Trace("[ext_misc_print_utf8_version_1] executing...")
	ptr, size := int64ToPointerAndSize(int64(data))
	ext_print_utf8(context, C.int32_t(ptr), C.int32_t(size))
}

//export ext_misc_runtime_version_version_1
func ext_misc_runtime_version_version_1(context unsafe.Pointer, z C.int64_t) C.int64_t {
	logger.Trace("[ext_misc_runtime_version_version_1] executing...")
	return 0
}

//export ext_default_child_storage_read_version_1
func ext_default_child_storage_read_version_1(context unsafe.Pointer, a C.int64_t, b C.int64_t, c C.int64_t, d C.int32_t) C.int64_t {
	logger.Trace("[ext_default_child_storage_read_version_1] executing...")
	return 0
}

//export ext_default_child_storage_clear_version_1
func ext_default_child_storage_clear_version_1(context unsafe.Pointer, a, b C.int64_t) {
	logger.Trace("[ext_default_child_storage_clear_version_1] executing...")
}

//export ext_default_child_storage_clear_prefix_version_1
func ext_default_child_storage_clear_prefix_version_1(context unsafe.Pointer, a C.int64_t, b C.int64_t) {
	logger.Trace("[ext_default_child_storage_clear_prefix_version_1] executing...")
}

//export ext_default_child_storage_exists_version_1
func ext_default_child_storage_exists_version_1(context unsafe.Pointer, a C.int64_t, b C.int64_t) C.int32_t {
	logger.Trace("[ext_default_child_storage_exists_version_1] executing...")
	return 0
}

//export ext_default_child_storage_get_version_1
func ext_default_child_storage_get_version_1(context unsafe.Pointer, a, b C.int64_t) C.int64_t {
	logger.Trace("[ext_default_child_storage_get_version_1] executing...")
	return 0
}

//export ext_default_child_storage_next_key_version_1
func ext_default_child_storage_next_key_version_1(context unsafe.Pointer, a C.int64_t, b C.int64_t) C.int64_t {
	logger.Trace("[ext_default_child_storage_next_key_version_1] executing...")
	return 0
}

//export ext_default_child_storage_root_version_1
func ext_default_child_storage_root_version_1(context unsafe.Pointer, z C.int64_t) C.int64_t {
	logger.Trace("[ext_default_child_storage_root_version_1] executing...")
	return 0
}

//export ext_default_child_storage_set_version_1
func ext_default_child_storage_set_version_1(context unsafe.Pointer, a, b, z C.int64_t) {
	logger.Trace("[ext_default_child_storage_set_version_1] executing...")
}

//export ext_default_child_storage_storage_kill_version_1
func ext_default_child_storage_storage_kill_version_1(context unsafe.Pointer, a C.int64_t) {
	logger.Trace("[ext_default_child_storage_storage_kill_version_1] executing...")
}

//export ext_allocator_free_version_1
func ext_allocator_free_version_1(context unsafe.Pointer, addr C.int32_t) {
	logger.Trace("[ext_allocator_free_version_1] executing...")
	ext_free(context, addr)
}

//export ext_allocator_malloc_version_1
func ext_allocator_malloc_version_1(context unsafe.Pointer, size C.int32_t) C.int32_t {
	logger.Trace("[ext_allocator_malloc_version_1] executing...", "size", size)
	return ext_malloc(context, size)
}

//export ext_hashing_blake2_128_version_1
func ext_hashing_blake2_128_version_1(context unsafe.Pointer, dataSpan C.int64_t) C.int32_t {
	logger.Trace("[ext_hashing_blake2_128_version_1] executing...")
	instanceContext := wasm.IntoInstanceContext(context)

	data := asMemorySlice(instanceContext, dataSpan)

	hash, err := common.Blake2b128(data)
	if err != nil {
		logger.Error("[ext_hashing_blake2_128_version_1]", "error", err)
		panic(err)
	}

	out, err := toWasmMemorySized(instanceContext, hash, 16)
	if err != nil {
		logger.Error("[ext_hashing_blake2_128_version_1] failed to allocate", "error", err)
		panic(err)
	}

	return C.int32_t(out)
}

//export ext_hashing_blake2_256_version_1
func ext_hashing_blake2_256_version_1(context unsafe.Pointer, dataSpan C.int64_t) C.int32_t {
	logger.Trace("[ext_hashing_blake2_256_version_1] executing...")
	instanceContext := wasm.IntoInstanceContext(context)

	data := asMemorySlice(instanceContext, dataSpan)

	hash, err := common.Blake2bHash(data)
	if err != nil {
		logger.Error("[ext_hashing_blake2_256_version_1]", "error", err)
		panic(err)
	}

	out, err := toWasmMemorySized(instanceContext, hash[:], 32)
	if err != nil {
		logger.Error("[ext_hashing_blake2_256_version_1] failed to allocate", "error", err)
		panic(err)
	}

	return C.int32_t(out)
}

//export ext_hashing_keccak_256_version_1
func ext_hashing_keccak_256_version_1(context unsafe.Pointer, dataSpan C.int64_t) C.int32_t {
	logger.Trace("[ext_hashing_keccak_256_version_1] executing...")
	instanceContext := wasm.IntoInstanceContext(context)

	data := asMemorySlice(instanceContext, dataSpan)

	hash, err := common.Keccak256(data)
	if err != nil {
		logger.Error("[ext_hashing_keccak_256_version_1]", "error", err)
		panic(err)
	}

	out, err := toWasmMemorySized(instanceContext, hash[:], 32)
	if err != nil {
		logger.Error("[ext_hashing_keccak_256_version_1] failed to allocate", "error", err)
		panic(err)
	}

	return C.int32_t(out)
}

//export ext_hashing_sha2_256_version_1
func ext_hashing_sha2_256_version_1(context unsafe.Pointer, z C.int64_t) C.int32_t {
	logger.Trace("[ext_hashing_sha2_256_version_1] executing...")
	return 0
}

//export ext_hashing_twox_256_version_1
func ext_hashing_twox_256_version_1(context unsafe.Pointer, dataSpan C.int64_t) C.int32_t {
	logger.Trace("[ext_hashing_twox_256_version_1] executing...")
	instanceContext := wasm.IntoInstanceContext(context)

	data := asMemorySlice(instanceContext, dataSpan)

	hash, err := common.Twox256(data)
	if err != nil {
		logger.Error("[ext_hashing_twox_256_version_1]", "error", err)
		panic(err)
	}

	out, err := toWasmMemorySized(instanceContext, hash[:], 32)
	if err != nil {
		logger.Error("[ext_hashing_twox_256_version_1] failed to allocate", "error", err)
		panic(err)
	}

	return C.int32_t(out)
}

//export ext_hashing_twox_128_version_1
func ext_hashing_twox_128_version_1(context unsafe.Pointer, data C.int64_t) C.int32_t {
	logger.Trace("[ext_hashing_twox_128_version_1] executing...")
	ptr, size := int64ToPointerAndSize(int64(data))

	instanceContext := wasm.IntoInstanceContext(context)
	ctx := instanceContext.Data().(*runtime.Context)
	out, err := ctx.Allocator.Allocate(16)
	if err != nil {
		logger.Error("[ext_hashing_twox_128_version_1] failed to allocate", "error", err)
		panic(err)
	}
	ext_twox_128(context, C.int32_t(ptr), C.int32_t(size), C.int32_t(out))
	return C.int32_t(out)
}

//export ext_hashing_twox_64_version_1
func ext_hashing_twox_64_version_1(context unsafe.Pointer, dataSpan C.int64_t) C.int32_t {
	logger.Trace("[ext_hashing_twox_64_version_1] executing...")
	instanceContext := wasm.IntoInstanceContext(context)

	data := asMemorySlice(instanceContext, dataSpan)

	hash, err := common.Twox64(data)
	if err != nil {
		logger.Error("[ext_hashing_twox_64_version_1]", "error", err)
		panic(err)
	}

	out, err := toWasmMemorySized(instanceContext, hash, 8)
	if err != nil {
		logger.Error("[ext_hashing_twox_64_version_1] failed to allocate", "error", err)
		panic(err)
	}

	return C.int32_t(out)
}

//export ext_offchain_index_set_version_1
func ext_offchain_index_set_version_1(context unsafe.Pointer, a, b C.int64_t) {
	logger.Trace("[ext_offchain_index_set_version_1] executing...")
}

//export ext_offchain_is_validator_version_1
func ext_offchain_is_validator_version_1(context unsafe.Pointer) C.int32_t {
	logger.Trace("[ext_offchain_is_validator_version_1] executing...")
	return 0
}

//export ext_offchain_local_storage_compare_and_set_version_1
func ext_offchain_local_storage_compare_and_set_version_1(context unsafe.Pointer, a C.int32_t, x, y, z C.int64_t) C.int32_t {
	logger.Trace("[ext_offchain_local_storage_compare_and_set_version_1] executing...")
	return 0
}

//export ext_offchain_local_storage_get_version_1
func ext_offchain_local_storage_get_version_1(context unsafe.Pointer, a C.int32_t, x C.int64_t) C.int64_t {
	logger.Trace("[ext_offchain_local_storage_get_version_1] executing...")
	return 0
}

//export ext_offchain_local_storage_set_version_1
func ext_offchain_local_storage_set_version_1(context unsafe.Pointer, a C.int32_t, x, y C.int64_t) {
	logger.Trace("[ext_offchain_local_storage_set_version_1] executing...")
}

//export ext_offchain_network_state_version_1
func ext_offchain_network_state_version_1(context unsafe.Pointer) C.int64_t {
	logger.Trace("[ext_offchain_network_state_version_1] executing...")
	return 0
}

//export ext_offchain_random_seed_version_1
func ext_offchain_random_seed_version_1(context unsafe.Pointer) C.int32_t {
	logger.Trace("[ext_offchain_random_seed_version_1] executing...")
	return 0
}

//export ext_offchain_submit_transaction_version_1
func ext_offchain_submit_transaction_version_1(context unsafe.Pointer, z C.int64_t) C.int64_t {
	logger.Trace("[ext_offchain_submit_transaction_version_1] executing...")
	return 0
}

//export ext_storage_append_version_1
func ext_storage_append_version_1(context unsafe.Pointer, a, b C.int64_t) {
	logger.Trace("[ext_storage_append_version_1] executing...")
}

//export ext_storage_changes_root_version_1
func ext_storage_changes_root_version_1(context unsafe.Pointer, z C.int64_t) C.int64_t {
	logger.Trace("[ext_storage_changes_root_version_1] executing...")
	return 0
}

//export ext_storage_clear_version_1
func ext_storage_clear_version_1(context unsafe.Pointer, a C.int64_t) {
	logger.Trace("[ext_storage_clear_version_1] executing...")
}

//export ext_storage_clear_prefix_version_1
func ext_storage_clear_prefix_version_1(context unsafe.Pointer, a C.int64_t) {
	logger.Trace("[ext_storage_clear_prefix_version_1] executing...")
}

//export ext_storage_commit_transaction_version_1
func ext_storage_commit_transaction_version_1(context unsafe.Pointer) {
	logger.Trace("[ext_storage_commit_transaction_version_1] executing...")
}

//export ext_storage_exists_version_1
func ext_storage_exists_version_1(context unsafe.Pointer, a C.int64_t) C.int32_t {
	logger.Trace("[ext_storage_exists_version_1] executing...")
	return 0
}

//export ext_storage_get_version_1
func ext_storage_get_version_1(context unsafe.Pointer, keySpan C.int64_t) C.int64_t {
	logger.Trace("[ext_storage_get_version_1] executing...")

	instanceContext := wasm.IntoInstanceContext(context)
	storage := instanceContext.Data().(*runtime.Context).Storage

	key := asMemorySlice(instanceContext, keySpan)

	logger.Trace("[ext_storage_get_version_1]", "key", fmt.Sprintf("0x%x", key))

	value, err := storage.Get(key)
	if err != nil {
		logger.Error("[ext_storage_get_version_1]", "error", err)
		return 0
	}
	logger.Trace("[ext_storage_get_version_1]", "value", value)

	valueSpan, err := toWasmMemoryOptional(instanceContext, value)
	if err != nil {
		logger.Error("[ext_storage_get_version_1] failed to allocate", "error", err)
		panic(err)
	}

	return C.int64_t(valueSpan)
}

//export ext_storage_next_key_version_1
func ext_storage_next_key_version_1(context unsafe.Pointer, keySpan C.int64_t) C.int64_t {
	logger.Trace("[ext_storage_next_key_version_1] executing...")

	instanceContext := wasm.IntoInstanceContext(context)
	storage := instanceContext.Data().(*runtime.Context).Storage

	key := asMemorySlice(instanceContext, keySpan)

	next := storage.NextKey(key)
	logger.Trace("[ext_storage_next_key_version_1]", "next", next)

	nextSpan, err := toWasmMemoryOptional(instanceContext, next)
	if err != nil {
		logger.Error("[ext_storage_next_key_version_1] failed to allocate", "error", err)
		panic(err)
	}

	return C.int64_t(nextSpan)
}

//export ext_storage_read_version_1
func ext_storage_read_version_1(context unsafe.Pointer, a, b C.int64_t, x C.int32_t) C.int64_t {
	logger.Trace("[ext_storage_read_version_1] executing...")
	return 0
}

//export ext_storage_rollback_transaction_version_1
func ext_storage_rollback_transaction_version_1(context unsafe.Pointer) {
	logger.Trace("[ext_storage_rollback_transaction_version_1] executing...")
}

//export ext_storage_root_version_1
func ext_storage_root_version_1(context unsafe.Pointer) C.int64_t {
	logger.Trace("[ext_storage_root_version_1] executing...")
	return 0
}

//export ext_storage_set_version_1
func ext_storage_set_version_1(context unsafe.Pointer, keySpan C.int64_t, valueSpan C.int64_t) {
	logger.Trace("[ext_storage_set_version_1] executing...")

	instanceContext := wasm.IntoInstanceContext(context)
	storage := instanceContext.Data().(*runtime.Context).Storage

	key := asMemorySlice(instanceContext, keySpan)
	value := asMemorySlice(instanceContext, valueSpan)

	logger.Trace("[ext_storage_set_version_1]", "key", fmt.Sprintf("0x%x", key), "val", value)
	err := storage.Set(key, value)
	if err != nil {
		logger.Error("[ext_storage_set_version_1]", "error", err)
		panic(err)
	}
}

//export ext_storage_start_transaction_version_1
func ext_storage_start_transaction_version_1(context unsafe.Pointer) {
	logger.Trace("[ext_storage_start_transaction_version_1] executing...")
}

// Convert 64bit wasm span descriptor to Go memory slice
func asMemorySlice(context wasm.InstanceContext, span C.int64_t) []byte {
	memory := context.Memory().Data()

	ptr, size := int64ToPointerAndSize(int64(span))

	return memory[ptr : ptr+size]
}

// Copy a byte slice to wasm memory and return the resulting 64bit span descriptor
func toWasmMemory(context wasm.InstanceContext, data []byte) (int64, error) {
	memory := context.Memory().Data()
	allocator := context.Data().(*runtime.Context).Allocator

	size := uint32(len(data))

	out, err := allocator.Allocate(size)
	if err != nil {
		return 0, err
	}

	copy(memory[out:out+size], data[:])

	return pointerAndSizeToInt64(int32(out), int32(size)), nil
}

// Copy a byte slice of a fixed size to wasm memory and return resulting pointer
func toWasmMemorySized(context wasm.InstanceContext, data []byte, size uint32) (uint32, error) {

	if int(size) != len(data) {
		return 0, errors.New("internal byte array size missmatch")
	}

	memory := context.Memory().Data()
	allocator := context.Data().(*runtime.Context).Allocator

	out, err := allocator.Allocate(size)
	if err != nil {
		return 0, err
	}

	copy(memory[out:out+size], data[:])

	return out, nil
}

// Wraps slice in optional and copies result to wasm memory. Returns resulting 64bit span descriptor
func toWasmMemoryOptional(context wasm.InstanceContext, data []byte) (int64, error) {

	var opt *optional.Bytes
	if len(data) == 0 {
		opt = optional.NewBytes(false, nil)
	} else {
		opt = optional.NewBytes(true, data)
	}

	enc, err := opt.Encode()
	if err != nil {
		return 0, err
	}

	return toWasmMemory(context, enc)
}

// ImportsNodeRuntime returns the imports for the v0.8 runtime
func ImportsNodeRuntime() (*wasm.Imports, error) { //nolint
	var err error

	imports := wasm.NewImports()

	_, err = imports.Append("ext_allocator_free_version_1", ext_allocator_free_version_1, C.ext_allocator_free_version_1)
	if err != nil {
		return nil, err
	}
	_, err = imports.Append("ext_allocator_malloc_version_1", ext_allocator_malloc_version_1, C.ext_allocator_malloc_version_1)
	if err != nil {
		return nil, err
	}

	_, err = imports.Append("ext_crypto_ed25519_generate_version_1", ext_crypto_ed25519_generate_version_1, C.ext_crypto_ed25519_generate_version_1)
	if err != nil {
		return nil, err
	}
	_, err = imports.Append("ext_crypto_ed25519_public_keys_version_1", ext_crypto_ed25519_public_keys_version_1, C.ext_crypto_ed25519_public_keys_version_1)
	if err != nil {
		return nil, err
	}
	_, err = imports.Append("ext_crypto_ed25519_sign_version_1", ext_crypto_ed25519_sign_version_1, C.ext_crypto_ed25519_sign_version_1)
	if err != nil {
		return nil, err
	}
	_, err = imports.Append("ext_crypto_ed25519_verify_version_1", ext_crypto_ed25519_verify_version_1, C.ext_crypto_ed25519_verify_version_1)
	if err != nil {
		return nil, err
	}
	_, err = imports.Append("ext_crypto_finish_batch_verify_version_1", ext_crypto_finish_batch_verify_version_1, C.ext_crypto_finish_batch_verify_version_1)
	if err != nil {
		return nil, err
	}
	_, err = imports.Append("ext_crypto_secp256k1_ecdsa_recover_version_1", ext_crypto_secp256k1_ecdsa_recover_version_1, C.ext_crypto_secp256k1_ecdsa_recover_version_1)
	if err != nil {
		return nil, err
	}
	_, err = imports.Append("ext_crypto_secp256k1_ecdsa_recover_compressed_version_1", ext_crypto_secp256k1_ecdsa_recover_compressed_version_1, C.ext_crypto_secp256k1_ecdsa_recover_compressed_version_1)
	if err != nil {
		return nil, err
	}
	_, err = imports.Append("ext_crypto_sr25519_generate_version_1", ext_crypto_sr25519_generate_version_1, C.ext_crypto_sr25519_generate_version_1)
	if err != nil {
		return nil, err
	}
	_, err = imports.Append("ext_crypto_sr25519_public_keys_version_1", ext_crypto_sr25519_public_keys_version_1, C.ext_crypto_sr25519_public_keys_version_1)
	if err != nil {
		return nil, err
	}
	_, err = imports.Append("ext_crypto_sr25519_sign_version_1", ext_crypto_sr25519_sign_version_1, C.ext_crypto_sr25519_sign_version_1)
	if err != nil {
		return nil, err
	}
	_, err = imports.Append("ext_crypto_sr25519_verify_version_1", ext_crypto_sr25519_verify_version_1, C.ext_crypto_sr25519_verify_version_1)
	if err != nil {
		return nil, err
	}
	_, err = imports.Append("ext_crypto_sr25519_verify_version_2", ext_crypto_sr25519_verify_version_2, C.ext_crypto_sr25519_verify_version_2)
	if err != nil {
		return nil, err
	}
	_, err = imports.Append("ext_crypto_start_batch_verify_version_1", ext_crypto_start_batch_verify_version_1, C.ext_crypto_start_batch_verify_version_1)
	if err != nil {
		return nil, err
	}

	_, err = imports.Append("ext_default_child_storage_clear_version_1", ext_default_child_storage_clear_version_1, C.ext_default_child_storage_clear_version_1)
	if err != nil {
		return nil, err
	}
	_, err = imports.Append("ext_default_child_storage_clear_prefix_version_1", ext_default_child_storage_clear_prefix_version_1, C.ext_default_child_storage_clear_prefix_version_1)
	if err != nil {
		return nil, err
	}
	_, err = imports.Append("ext_default_child_storage_exists_version_1", ext_default_child_storage_exists_version_1, C.ext_default_child_storage_exists_version_1)
	if err != nil {
		return nil, err
	}
	_, err = imports.Append("ext_default_child_storage_get_version_1", ext_default_child_storage_get_version_1, C.ext_default_child_storage_get_version_1)
	if err != nil {
		return nil, err
	}
	_, err = imports.Append("ext_default_child_storage_next_key_version_1", ext_default_child_storage_next_key_version_1, C.ext_default_child_storage_next_key_version_1)
	if err != nil {
		return nil, err
	}
	_, err = imports.Append("ext_default_child_storage_read_version_1", ext_default_child_storage_read_version_1, C.ext_default_child_storage_read_version_1)
	if err != nil {
		return nil, err
	}
	_, err = imports.Append("ext_default_child_storage_root_version_1", ext_default_child_storage_root_version_1, C.ext_default_child_storage_root_version_1)
	if err != nil {
		return nil, err
	}
	_, err = imports.Append("ext_default_child_storage_set_version_1", ext_default_child_storage_set_version_1, C.ext_default_child_storage_set_version_1)
	if err != nil {
		return nil, err
	}
	_, err = imports.Append("ext_default_child_storage_storage_kill_version_1", ext_default_child_storage_storage_kill_version_1, C.ext_default_child_storage_storage_kill_version_1)
	if err != nil {
		return nil, err
	}

	_, err = imports.Append("ext_hashing_blake2_128_version_1", ext_hashing_blake2_128_version_1, C.ext_hashing_blake2_128_version_1)
	if err != nil {
		return nil, err
	}
	_, err = imports.Append("ext_hashing_blake2_256_version_1", ext_hashing_blake2_256_version_1, C.ext_hashing_blake2_256_version_1)
	if err != nil {
		return nil, err
	}
	_, err = imports.Append("ext_hashing_keccak_256_version_1", ext_hashing_keccak_256_version_1, C.ext_hashing_keccak_256_version_1)
	if err != nil {
		return nil, err
	}
	_, err = imports.Append("ext_hashing_sha2_256_version_1", ext_hashing_sha2_256_version_1, C.ext_hashing_sha2_256_version_1)
	if err != nil {
		return nil, err
	}
	_, err = imports.Append("ext_hashing_twox_256_version_1", ext_hashing_twox_256_version_1, C.ext_hashing_twox_256_version_1)
	if err != nil {
		return nil, err
	}
	_, err = imports.Append("ext_hashing_twox_128_version_1", ext_hashing_twox_128_version_1, C.ext_hashing_twox_128_version_1)
	if err != nil {
		return nil, err
	}
	_, err = imports.Append("ext_hashing_twox_64_version_1", ext_hashing_twox_64_version_1, C.ext_hashing_twox_64_version_1)
	if err != nil {
		return nil, err
	}

	_, err = imports.Append("ext_logging_log_version_1", ext_logging_log_version_1, C.ext_logging_log_version_1)
	if err != nil {
		return nil, err
	}

	_, err = imports.Append("ext_misc_print_hex_version_1", ext_misc_print_hex_version_1, C.ext_misc_print_hex_version_1)
	if err != nil {
		return nil, err
	}
	_, err = imports.Append("ext_misc_print_num_version_1", ext_misc_print_num_version_1, C.ext_misc_print_num_version_1)
	if err != nil {
		return nil, err
	}
	_, err = imports.Append("ext_misc_print_utf8_version_1", ext_misc_print_utf8_version_1, C.ext_misc_print_utf8_version_1)
	if err != nil {
		return nil, err
	}
	_, err = imports.Append("ext_misc_runtime_version_version_1", ext_misc_runtime_version_version_1, C.ext_misc_runtime_version_version_1)
	if err != nil {
		return nil, err
	}

	_, err = imports.Append("ext_offchain_index_set_version_1", ext_offchain_index_set_version_1, C.ext_offchain_index_set_version_1)
	if err != nil {
		return nil, err
	}
	_, err = imports.Append("ext_offchain_is_validator_version_1", ext_offchain_is_validator_version_1, C.ext_offchain_is_validator_version_1)
	if err != nil {
		return nil, err
	}
	_, err = imports.Append("ext_offchain_local_storage_compare_and_set_version_1", ext_offchain_local_storage_compare_and_set_version_1, C.ext_offchain_local_storage_compare_and_set_version_1)
	if err != nil {
		return nil, err
	}
	_, err = imports.Append("ext_offchain_local_storage_get_version_1", ext_offchain_local_storage_get_version_1, C.ext_offchain_local_storage_get_version_1)
	if err != nil {
		return nil, err
	}
	_, err = imports.Append("ext_offchain_local_storage_set_version_1", ext_offchain_local_storage_set_version_1, C.ext_offchain_local_storage_set_version_1)
	if err != nil {
		return nil, err
	}
	_, err = imports.Append("ext_offchain_network_state_version_1", ext_offchain_network_state_version_1, C.ext_offchain_network_state_version_1)
	if err != nil {
		return nil, err
	}
	_, err = imports.Append("ext_offchain_random_seed_version_1", ext_offchain_random_seed_version_1, C.ext_offchain_random_seed_version_1)
	if err != nil {
		return nil, err
	}
	_, err = imports.Append("ext_offchain_submit_transaction_version_1", ext_offchain_submit_transaction_version_1, C.ext_offchain_submit_transaction_version_1)
	if err != nil {
		return nil, err
	}

	_, err = imports.Append("ext_sandbox_instance_teardown_version_1", ext_sandbox_instance_teardown_version_1, C.ext_sandbox_instance_teardown_version_1)
	if err != nil {
		return nil, err
	}
	_, err = imports.Append("ext_sandbox_instantiate_version_1", ext_sandbox_instantiate_version_1, C.ext_sandbox_instantiate_version_1)
	if err != nil {
		return nil, err
	}
	_, err = imports.Append("ext_sandbox_invoke_version_1", ext_sandbox_invoke_version_1, C.ext_sandbox_invoke_version_1)
	if err != nil {
		return nil, err
	}
	_, err = imports.Append("ext_sandbox_memory_get_version_1", ext_sandbox_memory_get_version_1, C.ext_sandbox_memory_get_version_1)
	if err != nil {
		return nil, err
	}
	_, err = imports.Append("ext_sandbox_memory_new_version_1", ext_sandbox_memory_new_version_1, C.ext_sandbox_memory_new_version_1)
	if err != nil {
		return nil, err
	}
	_, err = imports.Append("ext_sandbox_memory_set_version_1", ext_sandbox_memory_set_version_1, C.ext_sandbox_memory_set_version_1)
	if err != nil {
		return nil, err
	}
	_, err = imports.Append("ext_sandbox_memory_teardown_version_1", ext_sandbox_memory_teardown_version_1, C.ext_sandbox_memory_teardown_version_1)
	if err != nil {
		return nil, err
	}

	_, err = imports.Append("ext_storage_append_version_1", ext_storage_append_version_1, C.ext_storage_append_version_1)
	if err != nil {
		return nil, err
	}
	_, err = imports.Append("ext_storage_changes_root_version_1", ext_storage_changes_root_version_1, C.ext_storage_changes_root_version_1)
	if err != nil {
		return nil, err
	}
	_, err = imports.Append("ext_storage_clear_version_1", ext_storage_clear_version_1, C.ext_storage_clear_version_1)
	if err != nil {
		return nil, err
	}
	_, err = imports.Append("ext_storage_clear_prefix_version_1", ext_storage_clear_prefix_version_1, C.ext_storage_clear_prefix_version_1)
	if err != nil {
		return nil, err
	}
	_, err = imports.Append("ext_storage_commit_transaction_version_1", ext_storage_commit_transaction_version_1, C.ext_storage_commit_transaction_version_1)
	if err != nil {
		return nil, err
	}
	_, err = imports.Append("ext_storage_exists_version_1", ext_storage_exists_version_1, C.ext_storage_exists_version_1)
	if err != nil {
		return nil, err
	}
	_, err = imports.Append("ext_storage_get_version_1", ext_storage_get_version_1, C.ext_storage_get_version_1)
	if err != nil {
		return nil, err
	}
	_, err = imports.Append("ext_storage_next_key_version_1", ext_storage_next_key_version_1, C.ext_storage_next_key_version_1)
	if err != nil {
		return nil, err
	}
	_, err = imports.Append("ext_storage_read_version_1", ext_storage_read_version_1, C.ext_storage_read_version_1)
	if err != nil {
		return nil, err
	}
	_, err = imports.Append("ext_storage_rollback_transaction_version_1", ext_storage_rollback_transaction_version_1, C.ext_storage_rollback_transaction_version_1)
	if err != nil {
		return nil, err
	}
	_, err = imports.Append("ext_storage_root_version_1", ext_storage_root_version_1, C.ext_storage_root_version_1)
	if err != nil {
		return nil, err
	}
	_, err = imports.Append("ext_storage_set_version_1", ext_storage_set_version_1, C.ext_storage_set_version_1)
	if err != nil {
		return nil, err
	}
	_, err = imports.Append("ext_storage_start_transaction_version_1", ext_storage_start_transaction_version_1, C.ext_storage_start_transaction_version_1)
	if err != nil {
		return nil, err
	}

	_, err = imports.Append("ext_trie_blake2_256_ordered_root_version_1", ext_trie_blake2_256_ordered_root_version_1, C.ext_trie_blake2_256_ordered_root_version_1)
	if err != nil {
		return nil, err
	}
	_, err = imports.Append("ext_trie_blake2_256_root_version_1", ext_trie_blake2_256_root_version_1, C.ext_trie_blake2_256_root_version_1)
	if err != nil {
		return nil, err
	}

	return imports, nil
}
