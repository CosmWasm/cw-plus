// Copyright 2020 ChainSafe Systems (ON) Corp.
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

package rpc

import (
	"fmt"
	"log"
	"os"
	"testing"
	"time"

	"github.com/ChainSafe/gossamer/dot/rpc/modules"
	"github.com/ChainSafe/gossamer/tests/utils"
	"github.com/gorilla/websocket"
	"github.com/stretchr/testify/require"
)

func TestChainRPC(t *testing.T) {
	if utils.MODE != rpcSuite {
		_, _ = fmt.Fprintln(os.Stdout, "Going to skip RPC suite tests")
		return
	}

	testCases := []*testCase{
		{
			description: "test chain_getHeader",
			method:      "chain_getHeader",
			expected: modules.ChainBlockHeaderResponse{
				Number: "1",
			},
			params: "[]",
		},
		{
			description: "test chain_getBlock",
			method:      "chain_getBlock",
			expected: modules.ChainBlockResponse{
				Block: modules.ChainBlock{
					Header: modules.ChainBlockHeaderResponse{
						Number: "1",
					},
					Body: []string{},
				},
			},
			params: "[]",
		},
		{
			description: "test chain_getBlockHash",
			method:      "chain_getBlockHash",
			expected:    "",
			params:      "[]",
		},
		{
			description: "test chain_getFinalizedHead",
			method:      "chain_getFinalizedHead",
			expected:    "",
			params:      "[]",
		},
	}

	utils.CreateConfigBabeMaxThreshold()
	defer os.Remove(utils.ConfigBABEMaxThreshold)

	t.Log("starting gossamer...")
	nodes, err := utils.InitializeAndStartNodes(t, 1, utils.GenesisDefault, utils.ConfigBABEMaxThreshold)
	require.Nil(t, err)

	time.Sleep(time.Second) // give server a second to start

	chainBlockHeaderHash := ""
	for _, test := range testCases {

		t.Run(test.description, func(t *testing.T) {

			// set params for chain_getBlock from previous chain_getHeader call
			if chainBlockHeaderHash != "" {
				test.params = "[\"" + chainBlockHeaderHash + "\"]"
			}

			target := getResponse(t, test)

			switch v := target.(type) {
			case *modules.ChainBlockHeaderResponse:
				t.Log("Will assert ChainBlockHeaderResponse", "value", v)

				require.GreaterOrEqual(t, test.expected.(modules.ChainBlockHeaderResponse).Number, v.Number)

				require.NotNil(t, test.expected.(modules.ChainBlockHeaderResponse).ParentHash)
				require.NotNil(t, test.expected.(modules.ChainBlockHeaderResponse).StateRoot)
				require.NotNil(t, test.expected.(modules.ChainBlockHeaderResponse).ExtrinsicsRoot)
				require.NotNil(t, test.expected.(modules.ChainBlockHeaderResponse).Digest)

				//save for chain_getBlock
				chainBlockHeaderHash = v.ParentHash
			case *modules.ChainBlockResponse:
				t.Log("Will assert ChainBlockResponse", "value", v.Block)

				//reset
				chainBlockHeaderHash = ""

				require.NotNil(t, test.expected.(modules.ChainBlockResponse).Block)

				require.GreaterOrEqual(t, test.expected.(modules.ChainBlockResponse).Block.Header.Number, v.Block.Header.Number)

				require.NotNil(t, test.expected.(modules.ChainBlockResponse).Block.Header.ParentHash)
				require.NotNil(t, test.expected.(modules.ChainBlockResponse).Block.Header.StateRoot)
				require.NotNil(t, test.expected.(modules.ChainBlockResponse).Block.Header.ExtrinsicsRoot)
				require.NotNil(t, test.expected.(modules.ChainBlockResponse).Block.Header.Digest)

				require.NotNil(t, test.expected.(modules.ChainBlockResponse).Block.Body)
				require.GreaterOrEqual(t, len(test.expected.(modules.ChainBlockResponse).Block.Body), 0)

			case *string:
				t.Log("Will assert ChainBlockNumberRequest", "value", *v)
				require.NotNil(t, v)
				require.GreaterOrEqual(t, len(*v), 66)

			}

		})
	}

	t.Log("going to tear down gossamer...")
	errList := utils.TearDown(t, nodes)
	require.Len(t, errList, 0)
}

func TestChainSubscriptionRPC(t *testing.T) {
	if utils.MODE != rpcSuite {
		_, _ = fmt.Fprintln(os.Stdout, "Going to skip RPC suite tests")
		return
	}

	testCases := []*testCase{
		{
			description: "test chain_subscribeNewHeads",
			method:      "chain_subscribeNewHeads",
			expected: []interface{}{1,
				map[string](interface{}){
					"subscription": float64(1),
					"result": map[string](interface{}){
						"number":         "0x01",
						"parentHash":     "0x580d77a9136035a0bc3c3cd86286172f7f81291164c5914266073a30466fba21",
						"stateRoot":      "0x3b1a31d10d4d8a444579fd5a3fb17cbe6bebba9d939d88fe7bafb9d48036abb5",
						"extrinsicsRoot": "0x8025c0d64df303f79647611c8c2b0a77bc2247ee12d851df4624e1f71ebb3aed",
						"digest": map[string](interface{}){"logs": []interface{}{
							"0x0642414245c101c809062df1d1271d6a50232754baa64870515a7ada927886467748a220972c6d58347fd7317e286045604c5ddb78b84018c4b3a3836ee6626c8da6957338720053588d9f29c307fade658661d8d6a57c525f48553a253cf6e1475dbd319ca90200000000000000000e00000000000000",
							"0x054241424501017cac567e5b5688260d9d0a1f7fe6a9f81ae0f1900a382e1c73a4929fcaf6e33ed9e7347eb81ebb2699d58f6c8b01c7bdf0714e5f6f4495bc4b5fb3becb287580"}}}}},
			params: "[]",
			skip:   false,
		},
		{
			description: "test state_subscribeStorage",
			method:      "state_subscribeStorage",
			expected:    "",
			params:      "[]",
			skip:        true,
		},
		{
			description: "test chain_finalizedHeads",
			method:      "chain_subscribeFinalizedHeads",
			expected: []interface{}{1,
				map[string](interface{}){
					"subscription": float64(1),
					"result": map[string](interface{}){
						"number":         "0x01",
						"parentHash":     "0x580d77a9136035a0bc3c3cd86286172f7f81291164c5914266073a30466fba21",
						"stateRoot":      "0x3b1a31d10d4d8a444579fd5a3fb17cbe6bebba9d939d88fe7bafb9d48036abb5",
						"extrinsicsRoot": "0x8025c0d64df303f79647611c8c2b0a77bc2247ee12d851df4624e1f71ebb3aed",
						"digest": map[string](interface{}){"logs": []interface{}{
							"0x0642414245c101c809062df1d1271d6a50232754baa64870515a7ada927886467748a220972c6d58347fd7317e286045604c5ddb78b84018c4b3a3836ee6626c8da6957338720053588d9f29c307fade658661d8d6a57c525f48553a253cf6e1475dbd319ca90200000000000000000e00000000000000",
							"0x054241424501017cac567e5b5688260d9d0a1f7fe6a9f81ae0f1900a382e1c73a4929fcaf6e33ed9e7347eb81ebb2699d58f6c8b01c7bdf0714e5f6f4495bc4b5fb3becb287580"}}}}},
			params: "[]",
			skip:   false,
		},
	}

	utils.GenerateGenesisOneAuth()
	defer os.Remove(utils.GenesisOneAuth)
	utils.CreateConfigBabeMaxThreshold()
	defer os.Remove(utils.ConfigBABEMaxThreshold)

	t.Log("starting gossamer...")
	nodes, err := utils.InitializeAndStartNodesWebsocket(t, 1, utils.GenesisOneAuth, utils.ConfigBABEMaxThreshold)
	require.Nil(t, err)

	time.Sleep(time.Second) // give server a second to start

	for _, test := range testCases {

		t.Run(test.description, func(t *testing.T) {
			callWebsocket(t, test)
		})
	}

	time.Sleep(time.Second * 2)
	t.Log("going to tear down gossamer...")
	errList := utils.TearDown(t, nodes)
	require.Len(t, errList, 0)
}

func callWebsocket(t *testing.T, test *testCase) {
	if test.skip {
		t.Skip("Websocket endpoint not yet implemented")
	}
	url := "ws://localhost:8546/" // todo don't hard code this
	ws, _, err := websocket.DefaultDialer.Dial(url, nil)
	require.NoError(t, err)
	defer ws.Close()

	done := make(chan struct{})

	vals := make(chan []byte)
	go wsListener(t, ws, vals, done, len(test.expected.([]interface{})))

	err = ws.WriteMessage(websocket.TextMessage, []byte(`{
    "jsonrpc": "2.0",
    "method": "`+test.method+`",
    "params": [`+test.params+`],
    "id": 1
}`))
	require.NoError(t, err)
	resCount := 0
	for {
		select {
		case v := <-vals:
			resCount++
			switch exp := test.expected.([]interface{})[resCount-1].(type) {
			case int:
				// check for result subscription number
				resNum := 0
				err = utils.DecodeWebsocket(t, v, &resNum)
				require.NoError(t, err)

			case map[string]interface{}:
				// check result map response
				resMap := make(map[string]interface{})
				err = utils.DecodeWebsocket(t, v, &resMap)
				require.NoError(t, err)

				// check values in map are expected type
				for eKey, eVal := range exp {
					rVal := resMap[eKey]
					require.NotNil(t, rVal)
					require.IsType(t, eVal, rVal)
					switch evt := eVal.(type) {
					case map[string]interface{}:
						checkMap(t, evt, rVal.(map[string]interface{}))
					}
				}
			}

		case <-done:
			return
		}
	}
}

func wsListener(t *testing.T, ws *websocket.Conn, val chan []byte, done chan struct{}, msgCount int) {
	defer close(done)
	count := 0
	for {
		_, message, err := ws.ReadMessage()
		require.NoError(t, err)

		count++
		log.Printf("recv: %v: %s\n", count, message)

		val <- message
		if count == msgCount {
			err := ws.WriteMessage(websocket.CloseMessage, websocket.FormatCloseMessage(websocket.CloseNormalClosure, ""))
			require.NoError(t, err)
			return
		}
	}
}

func checkMap(t *testing.T, expMap map[string]interface{}, ckMap map[string]interface{}) {
	for eKey, eVal := range expMap {
		cVal := ckMap[eKey]

		require.NotNil(t, cVal)
		require.IsType(t, eVal, cVal)
		switch evt := eVal.(type) {
		case map[string]interface{}:
			checkMap(t, evt, cVal.(map[string]interface{}))
		}
	}

}
