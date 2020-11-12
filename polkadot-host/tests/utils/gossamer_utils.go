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

package utils

import (
	"bufio"
	"fmt"
	"io"
	"io/ioutil"
	"os"
	"os/exec"
	"path/filepath"
	"strconv"
	"sync"
	"testing"
	"time"

	"github.com/ChainSafe/gossamer/dot"
	ctoml "github.com/ChainSafe/gossamer/dot/config/toml"
	"github.com/ChainSafe/gossamer/dot/rpc/modules"
	"github.com/ChainSafe/gossamer/lib/utils"
	log "github.com/ChainSafe/log15"
)

var logger = log.New("pkg", "test/utils")
var maxRetries = 36

// SetLogLevel sets the logging level for this package
func SetLogLevel(lvl log.Lvl) {
	h := log.StreamHandler(os.Stdout, log.TerminalFormat())
	logger.SetHandler(log.LvlFilterHandler(log.LvlInfo, h))
}

var (
	keyList  = []string{"alice", "bob", "charlie", "dave", "eve", "ferdie", "george", "heather", "ian"}
	basePort = 7000

	// BaseRPCPort is the starting RPC port for test nodes
	BaseRPCPort = 8540

	// BaseWSPort is the starting Websocket port for test nodes
	BaseWSPort = 8546

	currentDir, _ = os.Getwd()
	gossamerCMD   = filepath.Join(currentDir, "../..", "bin/gossamer")

	// GenesisOneAuth is the genesis file that has 1 authority
	GenesisOneAuth string = filepath.Join(currentDir, "../utils/genesis_oneauth.json")
	// GenesisThreeAuths is the genesis file that has 3 authorities
	GenesisThreeAuths string = filepath.Join(currentDir, "../utils/genesis_threeauths.json")
	// GenesisSixAuths is the genesis file that has 6 authorities
	GenesisSixAuths string = filepath.Join(currentDir, "../utils/genesis_sixauths.json")
	// GenesisDefault is the default gssmr genesis file
	GenesisDefault string = filepath.Join(currentDir, "../..", "chain/gssmr/genesis-raw.json")

	// ConfigDefault is the default config file
	ConfigDefault string = filepath.Join(currentDir, "../..", "chain/gssmr/config.toml")
	// ConfigLogGrandpa is a config file where log levels are set to CRIT except for GRANDPA
	ConfigLogGrandpa string = filepath.Join(currentDir, "../utils/config_log_grandpa.toml")
	// ConfigNoBABE is a config file with BABE disabled
	ConfigNoBABE string = filepath.Join(currentDir, "../utils/config_nobabe.toml")
	// ConfigBABEMaxThreshold is a config file with BABE threshold set to maximum (node can produce block every slot)
	ConfigBABEMaxThreshold string = filepath.Join(currentDir, "../utils/config_babe_max_threshold.toml")
)

// Node represents a gossamer process
type Node struct {
	Process  *exec.Cmd
	Key      string
	RPCPort  string
	Idx      int
	basePath string
	config   string
	WSPort   string
}

// InitGossamer initializes given node number and returns node reference
func InitGossamer(idx int, basePath, genesis, config string) (*Node, error) {
	//nolint
	cmdInit := exec.Command(gossamerCMD, "init",
		"--config", config,
		"--basepath", basePath,
		"--genesis-raw", genesis,
		"--force",
	)

	//add step for init
	logger.Info("initializing gossamer...", "cmd", cmdInit)
	stdOutInit, err := cmdInit.CombinedOutput()
	if err != nil {
		fmt.Printf("%s", stdOutInit)
		return nil, err
	}

	// TODO: get init exit code to see if node was successfully initialized
	logger.Info("initialized gossamer!", "node", idx)

	return &Node{
		Idx:      idx,
		RPCPort:  strconv.Itoa(BaseRPCPort + idx),
		WSPort:   strconv.Itoa(BaseWSPort + idx),
		basePath: basePath,
		config:   config,
	}, nil
}

// StartGossamer starts given node
func StartGossamer(t *testing.T, node *Node, websocket bool) error {
	var key string
	var params []string = []string{"--port", strconv.Itoa(basePort + node.Idx),
		"--config", node.config,
		"--basepath", node.basePath,
		"--rpchost", HOSTNAME,
		"--rpcport", node.RPCPort,
		"--rpcmods", "system,author,chain,state,dev",
		"--rpc",
		"--log", "info"}

	if node.Idx >= len(keyList) {
		params = append(params, "--roles", "1")
	} else {
		key = keyList[node.Idx]
		params = append(params, "--roles", "4",
			"--key", key)
	}

	if websocket {
		params = append(params, "--ws",
			"--wsport", node.WSPort)
	}
	//nolint
	node.Process = exec.Command(gossamerCMD, params...)

	node.Key = key

	// create log file
	outfile, err := os.Create(filepath.Join(node.basePath, "log.out"))
	if err != nil {
		logger.Error("Error when trying to set a log file for gossamer output", "error", err)
		return err
	}

	// create error log file
	errfile, err := os.Create(filepath.Join(node.basePath, "error.out"))
	if err != nil {
		logger.Error("Error when trying to set a log file for gossamer output", "error", err)
		return err
	}

	t.Cleanup(func() {
		time.Sleep(time.Second) // wait for goroutine to finish writing
		outfile.Close()         //nolint
		errfile.Close()         //nolint
	})

	stdoutPipe, err := node.Process.StdoutPipe()
	if err != nil {
		logger.Error("failed to get stdoutPipe from node %d: %s\n", node.Idx, err)
		return err
	}

	stderrPipe, err := node.Process.StderrPipe()
	if err != nil {
		logger.Error("failed to get stderrPipe from node %d: %s\n", node.Idx, err)
		return err
	}

	logger.Info("starting gossamer...", "cmd", node.Process)
	err = node.Process.Start()
	if err != nil {
		logger.Error("Could not execute gossamer cmd", "err", err)
		return err
	}

	writer := bufio.NewWriter(outfile)
	go io.Copy(writer, stdoutPipe) //nolint
	errWriter := bufio.NewWriter(errfile)
	go io.Copy(errWriter, stderrPipe) //nolint

	var started bool
	for i := 0; i < maxRetries; i++ {
		time.Sleep(time.Second)
		if err = CheckNodeStarted(t, "http://"+HOSTNAME+":"+node.RPCPort); err == nil {
			started = true
			break
		}
	}

	if started {
		logger.Info("node started", "key", key, "cmd.Process.Pid", node.Process.Process.Pid)
	} else {
		logger.Crit("node didn't start!", "err", err)
		return err
	}

	return nil
}

// RunGossamer will initialize and start a gossamer instance
func RunGossamer(t *testing.T, idx int, basepath, genesis, config string, websocket bool) (*Node, error) {
	node, err := InitGossamer(idx, basepath, genesis, config)
	if err != nil {
		logger.Crit("could not initialize gossamer", "error", err)
		os.Exit(1)
	}

	err = StartGossamer(t, node, websocket)
	if err != nil {
		logger.Crit("could not start gossamer", "error", err)
		os.Exit(1)
	}

	return node, nil
}

// CheckNodeStarted check if gossamer node is started
func CheckNodeStarted(t *testing.T, gossamerHost string) error {
	method := "system_health"

	respBody, err := PostRPC(method, gossamerHost, "{}")
	if err != nil {
		return err
	}

	target := new(modules.SystemHealthResponse)
	err = DecodeRPC(t, respBody, target)
	if err != nil {
		return err
	}

	if !target.Health.ShouldHavePeers {
		return fmt.Errorf("no peers")
	}

	return nil
}

// KillProcess kills a instance of gossamer
func KillProcess(t *testing.T, cmd *exec.Cmd) error {
	err := cmd.Process.Kill()
	if err != nil {
		t.Log("failed to kill process", "cmd", cmd)
	}
	return err
}

// InitNodes initializes given number of nodes
func InitNodes(num int, config string) ([]*Node, error) {
	var nodes []*Node
	tempDir, err := ioutil.TempDir("", "gossamer-stress-")
	if err != nil {
		return nil, err
	}

	for i := 0; i < num; i++ {
		node, err := InitGossamer(i, tempDir+strconv.Itoa(i), GenesisDefault, config)
		if err != nil {
			logger.Error("failed to run gossamer", "i", i)
			return nil, err
		}

		nodes = append(nodes, node)
	}
	return nodes, nil
}

// StartNodes starts given array of nodes
func StartNodes(t *testing.T, nodes []*Node) error {
	for _, n := range nodes {
		err := StartGossamer(t, n, false)
		if err != nil {
			return nil
		}
	}
	return nil
}

// InitializeAndStartNodes will spin up `num` gossamer nodes
func InitializeAndStartNodes(t *testing.T, num int, genesis, config string) ([]*Node, error) {
	var nodes []*Node

	var wg sync.WaitGroup
	var nodeMu sync.Mutex
	wg.Add(num)

	for i := 0; i < num; i++ {
		go func(i int) {
			name := strconv.Itoa(i)
			if i < len(keyList) {
				name = keyList[i]
			}
			node, err := RunGossamer(t, i, TestDir(t, name), genesis, config, false)
			if err != nil {
				logger.Error("failed to run gossamer", "i", i)
			}

			nodeMu.Lock()
			nodes = append(nodes, node)
			nodeMu.Unlock()
			wg.Done()
		}(i)
	}

	wg.Wait()

	return nodes, nil
}

// InitializeAndStartNodesWebsocket will spin up `num` gossamer nodes running with Websocket rpc enabled
func InitializeAndStartNodesWebsocket(t *testing.T, num int, genesis, config string) ([]*Node, error) {
	var nodes []*Node

	var wg sync.WaitGroup
	wg.Add(num)

	for i := 0; i < num; i++ {
		go func(i int) {
			name := strconv.Itoa(i)
			if i < len(keyList) {
				name = keyList[i]
			}
			node, err := RunGossamer(t, i, TestDir(t, name), genesis, config, true)
			if err != nil {
				logger.Error("failed to run gossamer", "i", i)
			}

			nodes = append(nodes, node)
			wg.Done()
		}(i)
	}

	wg.Wait()

	return nodes, nil
}

// StopNodes stops the given nodes
func StopNodes(t *testing.T, nodes []*Node) (errs []error) {
	for i := range nodes {
		cmd := nodes[i].Process
		err := KillProcess(t, cmd)
		if err != nil {
			logger.Error("failed to kill gossamer", "i", i, "cmd", cmd)
			errs = append(errs, err)
		}
	}

	return errs
}

// TearDown stops the given nodes and remove their datadir
func TearDown(t *testing.T, nodes []*Node) (errorList []error) {
	for i, node := range nodes {
		cmd := nodes[i].Process
		err := KillProcess(t, cmd)
		if err != nil {
			logger.Error("failed to kill gossamer", "i", i, "cmd", cmd)
			errorList = append(errorList, err)
		}

		err = os.RemoveAll(node.basePath)
		if err != nil {
			logger.Error("failed to remove directory", "basepath", node.basePath)
			errorList = append(errorList, err)
		}
	}

	return errorList
}

// TestDir returns the test directory path <current-directory>/test_data/<test-name>/<name>
func TestDir(t *testing.T, name string) string {
	return filepath.Join("/tmp/", t.Name(), name)
}

// GenerateGenesisOneAuth generates Genesis file with one authority.
func GenerateGenesisOneAuth() {
	bs, err := dot.BuildFromGenesis(utils.GetGssmrGenesisPath(), 1)
	if err != nil {
		logger.Error("genesis file not found", "error", err)
		os.Exit(1)
	}
	_ = dot.CreateJSONRawFile(bs, GenesisOneAuth)
}

// GenerateGenesisThreeAuth generates Genesis file with three authority.
func GenerateGenesisThreeAuth() {
	bs, err := dot.BuildFromGenesis(utils.GetGssmrGenesisPath(), 3)
	if err != nil {
		logger.Error("genesis file not found", "error", err)
		os.Exit(1)
	}
	_ = dot.CreateJSONRawFile(bs, GenesisThreeAuths)
}

// GenerateGenesisSixAuth generates Genesis file with six authority.
func GenerateGenesisSixAuth() {
	bs, err := dot.BuildFromGenesis(utils.GetGssmrGenesisPath(), 6)
	if err != nil {
		logger.Error("genesis file not found", "error", err)
		os.Exit(1)
	}
	_ = dot.CreateJSONRawFile(bs, GenesisSixAuths)
}

func generateDefaultConfig() *ctoml.Config {
	return &ctoml.Config{
		Global: ctoml.GlobalConfig{
			Name:   "gssmr",
			ID:     "gssmr",
			LogLvl: "crit",
		},
		Log: ctoml.LogConfig{
			CoreLvl: "info",
			SyncLvl: "info",
		},
		Init: ctoml.InitConfig{
			GenesisRaw: "./chain/gssmr/genesis-raw.json",
		},
		Account: ctoml.AccountConfig{
			Key:    "",
			Unlock: "",
		},
		Core: ctoml.CoreConfig{
			Roles:            4,
			BabeAuthority:    true,
			GrandpaAuthority: true,
		},
		Network: ctoml.NetworkConfig{
			Bootnodes:   nil,
			ProtocolID:  "/gossamer/gssmr/0",
			NoBootstrap: false,
			NoMDNS:      false,
		},
		RPC: ctoml.RPCConfig{
			Enabled:   false,
			Host:      "localhost",
			Modules:   []string{"system", "author", "chain", "state"},
			WSEnabled: false,
		},
	}
}

func generateConfigBabeMaxThreshold() *ctoml.Config {
	cfg := generateDefaultConfig()
	cfg.Log = ctoml.LogConfig{
		SyncLvl:          "debug",
		NetworkLvl:       "debug",
		BlockProducerLvl: "info",
	}
	cfg.Core = ctoml.CoreConfig{
		Roles:            4,
		BabeAuthority:    true,
		GrandpaAuthority: true,
		BabeThreshold:    "max",
		SlotDuration:     500,
	}
	cfg.RPC.Modules = []string{"system", "author", "chain", "state", "dev"}
	return cfg
}

// CreateConfigBabeMaxThreshold generates and creates babe max threshold config file.
func CreateConfigBabeMaxThreshold() {
	cfg := generateConfigBabeMaxThreshold()
	_ = dot.ExportTomlConfig(cfg, ConfigBABEMaxThreshold)
}

func generateConfigLogGrandpa() *ctoml.Config {
	cfg := generateDefaultConfig()
	cfg.Log = ctoml.LogConfig{
		CoreLvl:           "crit",
		NetworkLvl:        "debug",
		RuntimeLvl:        "crit",
		BlockProducerLvl:  "info",
		FinalityGadgetLvl: "debug",
	}
	return cfg
}

// CreateConfigLogGrandpa generates and creates grandpa config file.
func CreateConfigLogGrandpa() {
	cfg := generateConfigLogGrandpa()
	_ = dot.ExportTomlConfig(cfg, ConfigLogGrandpa)
}

func generateConfigNoBabe() *ctoml.Config {
	cfg := generateDefaultConfig()
	cfg.Global.LogLvl = "info"
	cfg.Log = ctoml.LogConfig{
		SyncLvl:    "debug",
		NetworkLvl: "debug",
	}
	cfg.Core.BabeThreshold = "max"
	cfg.Core.BabeAuthority = false
	return cfg
}

// CreateConfigNoBabe generates and creates no babe config file.
func CreateConfigNoBabe() {
	cfg := generateConfigNoBabe()
	_ = dot.ExportTomlConfig(cfg, ConfigNoBABE)
}
