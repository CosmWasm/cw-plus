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
	"fmt"
	"os"
	"sync"

	"github.com/ChainSafe/gossamer/lib/runtime"
	log "github.com/ChainSafe/log15"
	wasm "github.com/wasmerio/go-ext-wasm/wasmer"
)

// Name represents the name of the interpreter
const Name = "wasmer"

// Check that runtime interfaces are satisfied
var _ runtime.LegacyInstance = (*LegacyInstance)(nil)
var _ runtime.Memory = (*wasm.Memory)(nil)

var logger = log.New("pkg", "runtime", "module", "go-wasmer")

// Config represents a wasmer configuration
type Config struct {
	runtime.InstanceConfig
	Imports func() (*wasm.Imports, error)
}

// LegacyInstance represents a v0.6 runtime go-wasmer instance
type LegacyInstance struct {
	vm    wasm.Instance
	ctx   *runtime.Context
	mutex sync.Mutex
}

// Instance represents a v0.8 runtime go-wasmer instance
type Instance struct {
	inst *LegacyInstance
}

// NewLegacyInstanceFromFile instantiates a runtime from a .wasm file
func NewLegacyInstanceFromFile(fp string, cfg *Config) (*LegacyInstance, error) {
	// Reads the WebAssembly module as bytes.
	bytes, err := wasm.ReadBytes(fp)
	if err != nil {
		return nil, err
	}

	return NewLegacyInstance(bytes, cfg)
}

// NewLegacyInstance instantiates a legacy runtime from raw wasm bytecode
func NewLegacyInstance(code []byte, cfg *Config) (*LegacyInstance, error) {
	return newLegacyInstance(code, cfg)
}

// NewInstanceFromFile instantiates a runtime from a .wasm file
func NewInstanceFromFile(fp string, cfg *Config) (*Instance, error) {
	// Reads the WebAssembly module as bytes.
	bytes, err := wasm.ReadBytes(fp)
	if err != nil {
		return nil, err
	}

	return NewInstance(bytes, cfg)
}

// NewInstance instantiates a runtime from raw wasm bytecode
func NewInstance(code []byte, cfg *Config) (*Instance, error) {
	inst, err := newLegacyInstance(code, cfg)
	if err != nil {
		return nil, err
	}

	// TODO: verify that v0.8 specific funcs are available
	return &Instance{
		inst: inst,
	}, nil
}

// Legacy returns the instance as a LegacyInstance
func (in *Instance) Legacy() *LegacyInstance {
	return in.inst
}

// SetContext sets the runtime's storage. It should be set before calls to the below functions.
func (in *Instance) SetContext(s runtime.Storage) {
	in.inst.SetContext(s)
}

// Stop func
func (in *Instance) Stop() {
	in.inst.Stop()
}

// Exec calls the given function with the given data
func (in *Instance) Exec(function string, data []byte) ([]byte, error) {
	return in.inst.Exec(function, data)
}

// Exec func
func (in *Instance) exec(function string, data []byte) ([]byte, error) {
	return in.inst.exec(function, data)
}

// NodeStorage to get reference to runtime node service
func (in *Instance) NodeStorage() runtime.NodeStorage {
	return in.inst.ctx.NodeStorage
}

// NetworkService to get referernce to runtime network service
func (in *Instance) NetworkService() runtime.BasicNetwork {
	return in.inst.ctx.Network
}

func newLegacyInstance(code []byte, cfg *Config) (*LegacyInstance, error) {
	// if cfg.LogLvl set to < 0, then don't change package log level
	if cfg.LogLvl >= 0 {
		h := log.StreamHandler(os.Stdout, log.TerminalFormat())
		h = log.CallerFileHandler(h)
		logger.SetHandler(log.LvlFilterHandler(cfg.LogLvl, h))
	}

	imports, err := cfg.Imports()
	if err != nil {
		return nil, err
	}

	// Provide importable memory for newer runtimes
	memory, err := wasm.NewMemory(20, 0)
	if err != nil {
		return nil, err
	}

	_, err = imports.AppendMemory("memory", memory)
	if err != nil {
		return nil, err
	}

	// Instantiates the WebAssembly module.
	instance, err := wasm.NewInstanceWithImports(code, imports)
	if err != nil {
		return nil, err
	}

	// Assume imported memory is used if runtime does not export any
	if !instance.HasMemory() {
		instance.Memory = memory
	}

	allocator := runtime.NewAllocator(instance.Memory, 0)

	runtimeCtx := &runtime.Context{
		Storage:     cfg.Storage,
		Allocator:   allocator,
		Keystore:    cfg.Keystore,
		Validator:   cfg.Role == byte(4),
		NodeStorage: cfg.NodeStorage,
		Network:     cfg.Network,
		Transaction: cfg.Transaction,
	}

	logger.Debug("NewInstance", "runtimeCtx", runtimeCtx)
	instance.SetContextData(runtimeCtx)

	return &LegacyInstance{
		vm:  instance,
		ctx: runtimeCtx,
	}, nil
}

// SetContext sets the runtime's storage. It should be set before calls to the below functions.
func (in *LegacyInstance) SetContext(s runtime.Storage) {
	in.ctx.Storage = s
	in.vm.SetContextData(in.ctx)
}

// Stop func
func (in *LegacyInstance) Stop() {
	in.vm.Close()
}

// Store func
func (in *LegacyInstance) store(data []byte, location int32) {
	mem := in.vm.Memory.Data()
	copy(mem[location:location+int32(len(data))], data)
}

// Load load
func (in *LegacyInstance) load(location, length int32) []byte {
	mem := in.vm.Memory.Data()
	return mem[location : location+length]
}

// Exec calls the given function with the given data
func (in *LegacyInstance) Exec(function string, data []byte) ([]byte, error) {
	return in.exec(function, data)
}

// Exec func
func (in *LegacyInstance) exec(function string, data []byte) ([]byte, error) {
	if in.ctx.Storage == nil {
		return nil, runtime.ErrNilStorage
	}

	ptr, err := in.malloc(uint32(len(data)))
	if err != nil {
		return nil, err
	}

	defer in.clear()

	in.mutex.Lock()
	defer in.mutex.Unlock()

	// Store the data into memory
	in.store(data, int32(ptr))
	datalen := int32(len(data))

	runtimeFunc, ok := in.vm.Exports[function]
	if !ok {
		return nil, fmt.Errorf("could not find exported function %s", function)
	}

	res, err := runtimeFunc(int32(ptr), datalen)
	if err != nil {
		return nil, err
	}

	offset, length := int64ToPointerAndSize(res.ToI64())
	return in.load(offset, length), nil
}

func (in *LegacyInstance) malloc(size uint32) (uint32, error) {
	return in.ctx.Allocator.Allocate(size)
}

func (in *LegacyInstance) clear() {
	in.ctx.Allocator.Clear()
}

// NodeStorage to get reference to runtime node service
func (in *LegacyInstance) NodeStorage() runtime.NodeStorage {
	return in.ctx.NodeStorage
}

// NetworkService to get referernce to runtime network service
func (in *LegacyInstance) NetworkService() runtime.BasicNetwork {
	return in.ctx.Network
}

// int64ToPointerAndSize converts an int64 into a int32 pointer and a int32 length
func int64ToPointerAndSize(in int64) (ptr int32, length int32) {
	return int32(in), int32(in >> 32)
}

// pointerAndSizeToInt64 converts int32 pointer and size to a int64
func pointerAndSizeToInt64(ptr, size int32) int64 {
	return int64(ptr) + (int64(size) << 32)
}
