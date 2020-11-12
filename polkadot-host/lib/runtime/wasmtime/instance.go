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

package wasmtime

import (
	"os"
	"runtime"
	"sync"

	gssmrruntime "github.com/ChainSafe/gossamer/lib/runtime"

	log "github.com/ChainSafe/log15"
	"github.com/bytecodealliance/wasmtime-go"
)

// Name represents the name of the interpreter
const Name = "wasmtime"

var _ gssmrruntime.LegacyInstance = (*LegacyInstance)(nil)

var ctx gssmrruntime.Context
var logger = log.New("pkg", "runtime", "module", "go-wasmtime")

// Config represents a wasmer configuration
type Config struct {
	gssmrruntime.InstanceConfig
	Imports func(*wasmtime.Store) []*wasmtime.Extern
}

// LegacyInstance represents a v0.6 runtime go-wasmtime instance
type LegacyInstance struct {
	vm  *wasmtime.Instance
	mu  sync.Mutex
	mem *wasmtime.Memory
}

// Instance represents a v0.8 runtime go-wasmtime instance
type Instance struct {
	inst *LegacyInstance
}

// NewLegacyInstance instantiates a runtime from the given wasm bytecode
func NewLegacyInstance(code []byte, cfg *Config) (*LegacyInstance, error) {
	engine := wasmtime.NewEngine()
	module, err := wasmtime.NewModule(engine, code)
	if err != nil {
		return nil, err
	}

	return newLegacyInstanceFromModule(module, engine, cfg)
}

// NewLegacyInstanceFromFile instantiates a runtime from a .wasm file
func NewLegacyInstanceFromFile(fp string, cfg *Config) (*LegacyInstance, error) {
	engine := wasmtime.NewEngine()
	module, err := wasmtime.NewModuleFromFile(engine, fp)
	if err != nil {
		return nil, err
	}

	return newLegacyInstanceFromModule(module, engine, cfg)
}

// NewInstanceFromFile instantiates a runtime from a .wasm file
func NewInstanceFromFile(fp string, cfg *Config) (*Instance, error) {
	inst, err := NewLegacyInstanceFromFile(fp, cfg)
	if err != nil {
		return nil, err
	}

	return &Instance{
		inst: inst,
	}, nil
}

func newLegacyInstanceFromModule(module *wasmtime.Module, engine *wasmtime.Engine, cfg *Config) (*LegacyInstance, error) {
	// if cfg.LogLvl set to < 0, then don't change package log level
	if cfg.LogLvl >= 0 {
		h := log.StreamHandler(os.Stdout, log.TerminalFormat())
		h = log.CallerFileHandler(h)
		logger.SetHandler(log.LvlFilterHandler(cfg.LogLvl, h))
	}

	store := wasmtime.NewStore(engine)
	instance, err := wasmtime.NewInstance(store, module, cfg.Imports(store))
	if err != nil {
		return nil, err
	}

	var mem *wasmtime.Memory
	if m := cfg.Imports(store)[0].Memory(); m != nil {
		mem = m
	} else {
		mem = instance.GetExport("memory").Memory()
	}

	allocator := gssmrruntime.NewAllocator(Memory{mem}, 0)

	ctx = gssmrruntime.Context{
		Storage:     cfg.Storage,
		Allocator:   allocator,
		Keystore:    cfg.Keystore,
		Validator:   cfg.Role == byte(4),
		NodeStorage: cfg.NodeStorage,
		Network:     cfg.Network,
	}

	return &LegacyInstance{
		vm:  instance,
		mem: mem,
	}, nil
}

// Legacy returns the instance as a LegacyInstance
func (in *Instance) Legacy() *LegacyInstance {
	return in.inst
}

// SetContext sets the runtime's storage. It should be set before calls to the below functions.
func (in *Instance) SetContext(s gssmrruntime.Storage) {
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
func (in *Instance) NodeStorage() gssmrruntime.NodeStorage {
	return ctx.NodeStorage
}

// NetworkService to get referernce to runtime network service
func (in *Instance) NetworkService() gssmrruntime.BasicNetwork {
	return ctx.Network
}

// SetContext sets the runtime context's Storage
func (in *LegacyInstance) SetContext(s gssmrruntime.Storage) {
	ctx.Storage = s
}

// Stop ...
func (in *LegacyInstance) Stop() {}

// NodeStorage returns the context's NodeStorage
func (in *LegacyInstance) NodeStorage() gssmrruntime.NodeStorage {
	return ctx.NodeStorage
}

// NetworkService returns the context's NetworkService
func (in *LegacyInstance) NetworkService() gssmrruntime.BasicNetwork {
	return ctx.Network
}

// Exec calls the given function with the given data
func (in *LegacyInstance) Exec(function string, data []byte) ([]byte, error) {
	return in.exec(function, data)
}

func (in *LegacyInstance) exec(function string, data []byte) ([]byte, error) {
	in.mu.Lock()
	defer in.mu.Unlock()

	ptr, err := ctx.Allocator.Allocate(uint32(len(data)))
	if err != nil {
		return nil, err
	}
	defer ctx.Allocator.Clear()

	memdata := in.mem.UnsafeData()
	copy(memdata[ptr:ptr+uint32(len(data))], data)

	run := in.vm.GetExport(function).Func()
	resi, err := run.Call(int32(ptr), int32(len(data)))
	if err != nil {
		return nil, err
	}

	if resi == nil {
		return []byte{}, err
	}

	ret := resi.(int64)
	length := int32(ret >> 32)
	offset := int32(ret)

	runtime.KeepAlive(in.mem)
	return memdata[offset : offset+length], nil
}
