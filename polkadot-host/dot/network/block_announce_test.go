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

package network

import (
	"math/big"
	"testing"

	"github.com/ChainSafe/gossamer/lib/common"
	"github.com/ChainSafe/gossamer/lib/utils"

	"github.com/libp2p/go-libp2p-core/peer"
	"github.com/stretchr/testify/require"
)

func TestBlockAnnounceDecoder(t *testing.T) {
	srv := &Service{
		blockAnnounceHandshakes: make(map[peer.ID]*blockAnnounceData),
	}

	testPeerID := peer.ID("QmaCpDMGvV2BGHeYERUEnRQAwe3N8SzbUtfsmvsqQLuvuJ")
	srv.blockAnnounceHandshakes[testPeerID] = &blockAnnounceData{
		received: false,
	}

	testHandshake := &BlockAnnounceHandshake{
		Roles:           4,
		BestBlockNumber: 77,
		BestBlockHash:   common.Hash{1},
		GenesisHash:     common.Hash{2},
	}

	enc, err := testHandshake.Encode()
	require.NoError(t, err)

	msg, err := srv.blockAnnounceDecoder(enc, testPeerID)
	require.NoError(t, err)
	require.Equal(t, testHandshake, msg)

	testBlockAnnounce := &BlockAnnounceMessage{
		ParentHash:     common.Hash{1},
		Number:         big.NewInt(77),
		StateRoot:      common.Hash{2},
		ExtrinsicsRoot: common.Hash{3},
		Digest:         [][]byte{},
	}

	enc, err = testBlockAnnounce.Encode()
	require.NoError(t, err)

	srv.blockAnnounceHandshakes[testPeerID].received = true
	msg, err = srv.blockAnnounceDecoder(enc, testPeerID)
	require.NoError(t, err)
	require.Equal(t, testBlockAnnounce, msg)
}

func TestHandleBlockAnnounceMessage_BlockAnnounce(t *testing.T) {
	basePath := utils.NewTestBasePath(t, "nodeA")

	// removes all data directories created within test directory
	defer utils.RemoveTestDir(t)

	config := &Config{
		BasePath:    basePath,
		Port:        7001,
		RandSeed:    1,
		NoBootstrap: true,
		NoMDNS:      true,
		NoStatus:    true,
	}

	s := createTestService(t, config)

	peerID := peer.ID("noot")
	msg := &BlockAnnounceMessage{
		Number: big.NewInt(10),
	}

	s.handleBlockAnnounceMessage(peerID, msg)
	require.True(t, s.requestTracker.hasRequestedBlockID(99))
}

func TestHandleBlockAnnounceMessage_BlockAnnounceHandshake(t *testing.T) {
	basePath := utils.NewTestBasePath(t, "nodeA")

	// removes all data directories created within test directory
	defer utils.RemoveTestDir(t)

	config := &Config{
		BasePath:    basePath,
		Port:        7001,
		RandSeed:    1,
		NoBootstrap: true,
		NoMDNS:      true,
		NoStatus:    true,
	}

	s := createTestService(t, config)

	testPeerID := peer.ID("noot")
	testHandshake := &BlockAnnounceHandshake{
		Roles:           4,
		BestBlockNumber: 77,
		BestBlockHash:   common.Hash{1},
		GenesisHash:     common.Hash{2},
	}

	s.handleBlockAnnounceMessage(testPeerID, testHandshake)
	require.True(t, s.blockAnnounceHandshakes[testPeerID].received)
	require.False(t, s.blockAnnounceHandshakes[testPeerID].validated)

	testHandshake = &BlockAnnounceHandshake{
		Roles:           4,
		BestBlockNumber: 77,
		BestBlockHash:   common.Hash{1},
		GenesisHash:     s.blockState.GenesisHash(),
	}

	s.handleBlockAnnounceMessage(testPeerID, testHandshake)
	require.True(t, s.blockAnnounceHandshakes[testPeerID].received)
	require.True(t, s.blockAnnounceHandshakes[testPeerID].validated)
}
