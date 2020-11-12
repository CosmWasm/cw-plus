package wasmer

import (
	"math/big"
	"testing"

	"github.com/ChainSafe/gossamer/dot/types"
	"github.com/ChainSafe/gossamer/lib/common"
	"github.com/ChainSafe/gossamer/lib/crypto/ed25519"
	"github.com/ChainSafe/gossamer/lib/runtime"
	"github.com/ChainSafe/gossamer/lib/trie"

	log "github.com/ChainSafe/log15"
	"github.com/stretchr/testify/require"
)

func TestInstance_Version_NodeRuntime(t *testing.T) {
	expected := &runtime.Version{
		Spec_name:         []byte("node"),
		Impl_name:         []byte("substrate-node"),
		Authoring_version: 10,
		Spec_version:      260,
		Impl_version:      0,
	}

	instance := NewTestInstance(t, runtime.NODE_RUNTIME)

	ret, err := instance.exec(runtime.CoreVersion, []byte{})
	require.Nil(t, err)

	version := &runtime.VersionAPI{
		RuntimeVersion: &runtime.Version{},
		API:            nil,
	}
	version.Decode(ret)
	require.Nil(t, err)

	t.Logf("Spec_name: %s\n", version.RuntimeVersion.Spec_name)
	t.Logf("Impl_name: %s\n", version.RuntimeVersion.Impl_name)
	t.Logf("Authoring_version: %d\n", version.RuntimeVersion.Authoring_version)
	t.Logf("Spec_version: %d\n", version.RuntimeVersion.Spec_version)
	t.Logf("Impl_version: %d\n", version.RuntimeVersion.Impl_version)

	require.Equal(t, expected, version.RuntimeVersion)
}

func TestInstance_GrandpaAuthorities_NodeRuntime(t *testing.T) {
	t.Skip()

	tt := trie.NewEmptyTrie()

	//value, err := common.HexToBytes("0x08eea1eabcac7d2c8a6459b7322cf997874482bfc3d2ec7a80888a3a7d714103640100000000000000b64994460e59b30364cad3c92e3df6052f9b0ebbb8f88460c194dc5794d6d7170100000000000000")
	value, err := common.HexToBytes("0x18dea6f4a727d3b2399275d6ee8817881f10597471dc1d27f144295ad6fb933c7a010000000000000048b623941c2a4d41cf25ef495408690fc853f777192498c0922eab1e9df4f0610100000000000000f72daf2e560e4f0f22fb5cbb04ad1d7fee850aab238fd014c178769e7e3a9b8401000000000000001c151c11cb72334d26d70769e3af7bbff3801a4e2dca2b09b7cce0af8dd813070100000000000000680d278213f908658a49a1025a7f466c197e8fb6fabb5e62220a7bd75f860cab01000000000000008e59368700ea89e2bf8922cc9e4b86d6651d1c689a0d57813f9768dbaadecf710100000000000000")
	require.NoError(t, err)

	err = tt.Put(runtime.GrandpaAuthorityDataKey, value)
	require.NoError(t, err)

	rt := NewTestInstanceWithTrie(t, runtime.NODE_RUNTIME, tt, log.LvlTrace)

	auths, err := rt.GrandpaAuthorities()
	require.NoError(t, err)

	authABytes, _ := common.HexToBytes("0xeea1eabcac7d2c8a6459b7322cf997874482bfc3d2ec7a80888a3a7d71410364")
	authBBytes, _ := common.HexToBytes("0xb64994460e59b30364cad3c92e3df6052f9b0ebbb8f88460c194dc5794d6d717")

	authA, _ := ed25519.NewPublicKey(authABytes)
	authB, _ := ed25519.NewPublicKey(authBBytes)

	expected := []*types.Authority{
		{Key: authA, Weight: 0},
		{Key: authB, Weight: 1},
	}

	require.Equal(t, expected, auths)
}

func TestInstance_BabeConfiguration_NodeRuntime_NoAuthorities(t *testing.T) {
	rt := NewTestInstance(t, runtime.NODE_RUNTIME)
	cfg, err := rt.BabeConfiguration()
	require.NoError(t, err)

	expected := &types.BabeConfiguration{
		SlotDuration:       3000,
		EpochLength:        200,
		C1:                 1,
		C2:                 4,
		GenesisAuthorities: nil,
		Randomness:         [32]byte{},
		SecondarySlots:     true,
	}

	require.Equal(t, expected, cfg)
}

func TestInstance_BabeConfiguration_NodeRuntime_WithAuthorities(t *testing.T) {
	t.Skip()

	tt := trie.NewEmptyTrie()

	// rvalue, err := common.HexToHash("0x01")
	// require.NoError(t, err)
	// err = tt.Put(runtime.BABERandomnessKey(), rvalue[:])
	// require.NoError(t, err)

	avalue, err := common.HexToBytes("0x18fa3437b10f6e7af8f31362df3a179b991a8c56313d1bcd6307a4d0c734c1ae310100000000000000d2419bc8835493ac89eb09d5985281f5dff4bc6c7a7ea988fd23af05f301580a0100000000000000ccb6bef60defc30724545d57440394ed1c71ea7ee6d880ed0e79871a05b5e40601000000000000005e67b64cf07d4d258a47df63835121423551712844f5b67de68e36bb9a21e12701000000000000006236877b05370265640c133fec07e64d7ca823db1dc56f2d3584b3d7c0f1615801000000000000006c52d02d95c30aa567fda284acf25025ca7470f0b0c516ddf94475a1807c4d250100000000000000")
	//avalue, err := common.HexToBytes("0x08eea1eabcac7d2c8a6459b7322cf997874482bfc3d2ec7a80888a3a7d714103640100000000000000b64994460e59b30364cad3c92e3df6052f9b0ebbb8f88460c194dc5794d6d7170100000000000000")
	require.NoError(t, err)

	err = tt.Put(runtime.BABEAuthorityDataKey(), avalue)
	require.NoError(t, err)

	rt := NewTestInstanceWithTrie(t, runtime.NODE_RUNTIME, tt, log.LvlTrace)

	cfg, err := rt.BabeConfiguration()
	require.NoError(t, err)

	authA, _ := common.HexToHash("0xeea1eabcac7d2c8a6459b7322cf997874482bfc3d2ec7a80888a3a7d71410364")
	authB, _ := common.HexToHash("0xb64994460e59b30364cad3c92e3df6052f9b0ebbb8f88460c194dc5794d6d717")

	expectedAuthData := []*types.AuthorityRaw{
		{Key: authA, Weight: 1},
		{Key: authB, Weight: 1},
	}

	expected := &types.BabeConfiguration{
		SlotDuration:       3000,
		EpochLength:        200,
		C1:                 1,
		C2:                 4,
		GenesisAuthorities: expectedAuthData,
		Randomness:         [32]byte{1},
		SecondarySlots:     true,
	}

	require.Equal(t, expected, cfg)
}

func TestInstance_ExecuteBlock_NodeRuntime(t *testing.T) {
	t.Skip()

	rt := NewTestInstance(t, runtime.NODE_RUNTIME)

	header := &types.Header{
		ParentHash:     common.Hash{},
		Number:         big.NewInt(1),
		StateRoot:      trie.EmptyHash,
		ExtrinsicsRoot: trie.EmptyHash,
		Digest:         [][]byte{},
	}

	_, err := rt.ExecuteBlock(&types.Block{
		Header: header,
		Body:   types.NewBody([]byte{}),
	})
	require.NoError(t, err)
}

func TestInstance_InitializeBlock_NodeRuntime(t *testing.T) {
	t.Skip()

	rt := NewTestInstance(t, runtime.NODE_RUNTIME)

	header := &types.Header{
		Number: big.NewInt(1),
		Digest: [][]byte{},
	}

	err := rt.InitializeBlock(header)
	require.NoError(t, err)
}
