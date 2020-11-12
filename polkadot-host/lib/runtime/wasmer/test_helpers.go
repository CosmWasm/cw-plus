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

import (
	"path/filepath"
	"testing"

	database "github.com/ChainSafe/chaindb"
	"github.com/ChainSafe/gossamer/lib/common"
	"github.com/ChainSafe/gossamer/lib/keystore"
	"github.com/ChainSafe/gossamer/lib/runtime"
	"github.com/ChainSafe/gossamer/lib/transaction"
	"github.com/ChainSafe/gossamer/lib/trie"
	log "github.com/ChainSafe/log15"
	"github.com/stretchr/testify/require"
	wasm "github.com/wasmerio/go-ext-wasm/wasmer"
)

var defaultTestLogLvl = log.LvlTrace

// NewTestLegacyInstance will create a new runtime instance using the given target runtime
func NewTestLegacyInstance(t *testing.T, targetRuntime string) *LegacyInstance {
	return NewTestLegacyInstanceWithTrie(t, targetRuntime, nil, defaultTestLogLvl)
}

// NewTestLegacyInstanceWithTrie will create a new runtime (polkadot/test) with the supplied trie as the storage
func NewTestLegacyInstanceWithTrie(t *testing.T, targetRuntime string, tt *trie.Trie, lvl log.Lvl) *LegacyInstance {
	fp, cfg := setupConfig(t, targetRuntime, tt, lvl, 0)
	r, err := NewLegacyInstanceFromFile(fp, cfg)
	require.NoError(t, err, "Got error when trying to create new VM", "targetRuntime", targetRuntime)
	require.NotNil(t, r, "Could not create new VM instance", "targetRuntime", targetRuntime)
	return r
}

// NewTestLegacyInstanceWithRole returns a test runtime with given role value
func NewTestLegacyInstanceWithRole(t *testing.T, targetRuntime string, role byte) *LegacyInstance {
	fp, cfg := setupConfig(t, targetRuntime, nil, defaultTestLogLvl, role)
	r, err := NewLegacyInstanceFromFile(fp, cfg)
	require.NoError(t, err, "Got error when trying to create new VM", "targetRuntime", targetRuntime)
	require.NotNil(t, r, "Could not create new VM instance", "targetRuntime", targetRuntime)
	return r
}

// NewTestInstance will create a new runtime instance using the given target runtime
func NewTestInstance(t *testing.T, targetRuntime string) *Instance {
	return NewTestInstanceWithTrie(t, targetRuntime, nil, defaultTestLogLvl)
}

// NewTestInstanceWithTrie will create a new runtime (polkadot/test) with the supplied trie as the storage
func NewTestInstanceWithTrie(t *testing.T, targetRuntime string, tt *trie.Trie, lvl log.Lvl) *Instance {
	fp, cfg := setupConfig(t, targetRuntime, tt, log.LvlTrace, 0)
	r, err := NewInstanceFromFile(fp, cfg)
	require.NoError(t, err, "Got error when trying to create new VM", "targetRuntime", targetRuntime)
	require.NotNil(t, r, "Could not create new VM instance", "targetRuntime", targetRuntime)
	return r
}

// NewTestInstanceWithRole returns a test runtime with given role value
func NewTestInstanceWithRole(t *testing.T, targetRuntime string, role byte) *Instance {
	fp, cfg := setupConfig(t, targetRuntime, nil, defaultTestLogLvl, role)
	r, err := NewInstanceFromFile(fp, cfg)
	require.NoError(t, err, "Got error when trying to create new VM", "targetRuntime", targetRuntime)
	require.NotNil(t, r, "Could not create new VM instance", "targetRuntime", targetRuntime)
	return r
}

// GetRuntimeImports ...
func GetRuntimeImports(targetRuntime string) func() (*wasm.Imports, error) {
	var registerImports func() (*wasm.Imports, error)

	switch targetRuntime {
	case runtime.SUBSTRATE_TEST_RUNTIME:
		registerImports = ImportsTestRuntime
	case runtime.LEGACY_NODE_RUNTIME:
		registerImports = ImportsLegacyNodeRuntime
	case runtime.NODE_RUNTIME:
		registerImports = ImportsNodeRuntime
	case runtime.TEST_RUNTIME:
		registerImports = ImportsLegacyNodeRuntime
	default:
		registerImports = ImportsLegacyNodeRuntime
	}

	return registerImports
}

func setupConfig(t *testing.T, targetRuntime string, tt *trie.Trie, lvl log.Lvl, role byte) (string, *Config) {
	testRuntimeFilePath, testRuntimeURL := runtime.GetRuntimeVars(targetRuntime)
	importsFunc := GetRuntimeImports(targetRuntime)

	_, err := runtime.GetRuntimeBlob(testRuntimeFilePath, testRuntimeURL)
	require.Nil(t, err, "Fail: could not get runtime", "targetRuntime", targetRuntime)

	s := runtime.NewTestRuntimeStorage(t, tt)

	fp, err := filepath.Abs(testRuntimeFilePath)
	require.Nil(t, err, "could not create testRuntimeFilePath", "targetRuntime", targetRuntime)

	ns := runtime.NodeStorage{
		LocalStorage:      database.NewMemDatabase(),
		PersistentStorage: database.NewMemDatabase(), // we're using a local storage here since this is a test runtime
	}
	cfg := &Config{
		Imports: importsFunc,
	}
	cfg.Storage = s
	cfg.Keystore = keystore.NewGenericKeystore("test")
	cfg.LogLvl = lvl
	cfg.NodeStorage = ns
	cfg.Network = new(runtime.TestRuntimeNetwork)
	cfg.Transaction = new(mockTransactionState)
	cfg.Role = role
	return fp, cfg
}

type mockTransactionState struct {
}

// AddToPool adds a transaction to the pool
func (mt *mockTransactionState) AddToPool(vt *transaction.ValidTransaction) common.Hash {
	return common.BytesToHash([]byte("test"))
}
