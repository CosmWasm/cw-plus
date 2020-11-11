#!/usr/bin/make -f

VERSION := $(shell echo $(shell git describe --tags) | sed 's/^v//')
COMMIT := $(shell git log -1 --format='%H')
PACKAGES=$(shell go list ./... | grep -Ev 'vendor|importer|rpc/tester')
DOCKER_TAG = unstable
DOCKER_IMAGE = aragon/aragon-chain
ARAGON_CHAIN_DAEMON_BINARY = aragonchaind
ARAGON_CHAIN_CLI_BINARY = aragonchaincli
GO_MOD=GO111MODULE=on
BUILDDIR ?= $(CURDIR)/build
SIMAPP = ./app
LEDGER_ENABLED ?= true

ifeq ($(DETECTED_OS),)
  ifeq ($(OS),Windows_NT)
	  DETECTED_OS := windows
  else
	  UNAME_S = $(shell uname -s)
    ifeq ($(UNAME_S),Darwin)
	    DETECTED_OS := mac
	  else
	    DETECTED_OS := linux
	  endif
  endif
endif
export GO111MODULE = on

##########################################
# Find OS and Go environment
# GO contains the Go binary
# FS contains the OS file separator
##########################################

ifeq ($(OS),Windows_NT)
  GO := $(shell where go.exe 2> NUL)
  FS := "\\"
else
  GO := $(shell command -v go 2> /dev/null)
  FS := "/"
endif

ifeq ($(GO),)
  $(error could not find go. Is it in PATH? $(GO))
endif

GOPATH ?= $(shell $(GO) env GOPATH)
BINDIR ?= $(GOPATH)/bin
RUNSIM = $(BINDIR)/runsim

# process build tags

build_tags = netgo
ifeq ($(LEDGER_ENABLED),true)
  ifeq ($(OS),Windows_NT)
    GCCEXE = $(shell where gcc.exe 2> NUL)
    ifeq ($(GCCEXE),)
      $(error gcc.exe not installed for ledger support, please install or set LEDGER_ENABLED=false)
    else
      build_tags += ledger
    endif
  else
    UNAME_S = $(shell uname -s)
    ifeq ($(UNAME_S),OpenBSD)
      $(warning OpenBSD detected, disabling ledger support (https://github.com/cosmos/cosmos-sdk/issues/1988))
    else
      GCC = $(shell command -v gcc 2> /dev/null)
      ifeq ($(GCC),)
        $(error gcc not installed for ledger support, please install or set LEDGER_ENABLED=false)
      else
        build_tags += ledger
      endif
    endif
  endif
endif

ifeq ($(WITH_CLEVELDB),yes)
  build_tags += gcc
endif
build_tags += $(BUILD_TAGS)
build_tags := $(strip $(build_tags))

whitespace :=
whitespace += $(whitespace)
comma := ,
build_tags_comma_sep := $(subst $(whitespace),$(comma),$(build_tags))

# process linker flags

ldflags = -X github.com/cosmos/cosmos-sdk/version.Name=aragon-chain \
		  -X github.com/cosmos/cosmos-sdk/version.ServerName=$(ARAGON_CHAIN_DAEMON_BINARY) \
		  -X github.com/cosmos/cosmos-sdk/version.ClientName=$(ARAGON_CHAIN_CLI_BINARY) \
		  -X github.com/cosmos/cosmos-sdk/version.Version=$(VERSION) \
		  -X github.com/cosmos/cosmos-sdk/version.Commit=$(COMMIT) \
		  -X "github.com/cosmos/cosmos-sdk/version.BuildTags=$(build_tags_comma_sep)"

ifeq ($(WITH_CLEVELDB),yes)
  ldflags += -X github.com/cosmos/cosmos-sdk/types.DBBackend=cleveldb
endif
ldflags += $(LDFLAGS)
ldflags := $(strip $(ldflags))

BUILD_FLAGS := -tags "$(build_tags)" -ldflags '$(ldflags)'

all: tools verify install

###############################################################################
###                                  Build                                  ###
###############################################################################

build: go.sum
ifeq ($(OS), Windows_NT)
	go build -mod=readonly $(BUILD_FLAGS) -o build/$(ARAGON_CHAIN_DAEMON_BINARY).exe ./cmd/$(ARAGON_CHAIN_DAEMON_BINARY)
	go build -mod=readonly $(BUILD_FLAGS) -o build/$(ARAGON_CHAIN_CLI_BINARY).exe ./cmd/$(ARAGON_CHAIN_CLI_BINARY)
else
	go build -mod=readonly $(BUILD_FLAGS) -o build/$(ARAGON_CHAIN_DAEMON_BINARY) ./cmd/$(ARAGON_CHAIN_DAEMON_BINARY)
	go build -mod=readonly $(BUILD_FLAGS) -o build/$(ARAGON_CHAIN_CLI_BINARY) ./cmd/$(ARAGON_CHAIN_CLI_BINARY)
endif
	go build -mod=readonly ./...

build-aragon-chain: go.sum
	mkdir -p $(BUILDDIR)
	go build -mod=readonly $(BUILD_FLAGS) -o $(BUILDDIR) ./cmd/$(ARAGON_CHAIN_DAEMON_BINARY)
	go build -mod=readonly $(BUILD_FLAGS) -o $(BUILDDIR) ./cmd/$(ARAGON_CHAIN_CLI_BINARY)

build-aragon-chain-linux: go.sum
	GOOS=linux GOARCH=amd64 CGO_ENABLED=1 $(MAKE) build-aragon-chain

.PHONY: build build-aragon-chain build-aragon-chain-linux

install:
	${GO_MOD} go install $(BUILD_FLAGS) ./cmd/$(ARAGON_CHAIN_DAEMON_BINARY)
	${GO_MOD} go install $(BUILD_FLAGS) ./cmd/$(ARAGON_CHAIN_CLI_BINARY)

clean:
	@rm -rf ./build ./vendor

docker-build:
	docker build -t ${DOCKER_IMAGE}:${DOCKER_TAG} .
	docker tag ${DOCKER_IMAGE}:${DOCKER_TAG} ${DOCKER_IMAGE}:latest
	docker tag ${DOCKER_IMAGE}:${DOCKER_TAG} ${DOCKER_IMAGE}:${COMMIT_HASH}
	# update old container
	docker rm aragonchain
	# create a new container from the latest image
	docker create --name aragonchain -t -i cosmos/aragonchain:latest aragonchain
	# move the binaries to the ./build directory
	mkdir -p ./build/
	docker cp aragonchain:/usr/bin/aragonchaind ./build/ ; \
	docker cp aragonchain:/usr/bin/aragonchaincli ./build/

docker-localnet:
	docker build -f ./networks/local/aragonchainnode/Dockerfile . -t aragonchaind/node

###############################################################################
###                          Tools & Dependencies                           ###
###############################################################################

TOOLS_DESTDIR  ?= $(GOPATH)/bin
RUNSIM         = $(TOOLS_DESTDIR)/runsim

# Install the runsim binary with a temporary workaround of entering an outside
# directory as the "go get" command ignores the -mod option and will polute the
# go.{mod, sum} files.
#
# ref: https://github.com/golang/go/issues/30515
runsim: $(RUNSIM)
$(RUNSIM):
	@echo "Installing runsim..."
	@(cd /tmp && go get github.com/cosmos/tools/cmd/runsim@v1.0.0)

tools: tools-stamp
tools-stamp: runsim
	# Create dummy file to satisfy dependency and avoid
	# rebuilding when this Makefile target is hit twice
	# in a row.
	touch $@

tools-clean:
	rm -f $(RUNSIM)
	rm -f tools-stamp

.PHONY: runsim tools tools-stamp tools-clean

###############################################################################
###                           Tests & Simulation                            ###
###############################################################################

test: test-unit

test-unit:
	@go test -v ./... $(PACKAGES)

test-race:
	@go test -v --vet=off -race ./... $(PACKAGES)

test-import:
	@go test ./importer -v --vet=off --run=TestImportBlocks --datadir tmp \
	--blockchain blockchain --timeout=10m
	rm -rf importer/tmp

test-rpc:
	./scripts/integration-test-all.sh -q 1 -z 1 -s 2

test-sim-nondeterminism:
	@echo "Running non-determinism test..."
	@go test -mod=readonly $(SIMAPP) -run TestAppStateDeterminism -Enabled=true \
		-NumBlocks=100 -BlockSize=200 -Commit=true -Period=0 -v -timeout 24h

test-sim-custom-genesis-fast:
	@echo "Running custom genesis simulation..."
	@echo "By default, ${HOME}/.$(ARAGON_CHAIN_DAEMON_BINARY)/config/genesis.json will be used."
	@go test -mod=readonly $(SIMAPP) -run TestFullAppSimulation -Genesis=${HOME}/.$(ARAGON_CHAIN_DAEMON_BINARY)/config/genesis.json \
		-Enabled=true -NumBlocks=100 -BlockSize=200 -Commit=true -Seed=99 -Period=5 -v -timeout 24h

test-sim-import-export: runsim
	@echo "Running application import/export simulation. This may take several minutes..."
	@$(BINDIR)/runsim -Jobs=4 -SimAppPkg=$(SIMAPP) -ExitOnFail 50 5 TestAppImportExport

test-sim-after-import: runsim
	@echo "Running application simulation-after-import. This may take several minutes..."
	@$(BINDIR)/runsim -Jobs=4 -SimAppPkg=$(SIMAPP) -ExitOnFail 50 5 TestAppSimulationAfterImport

test-sim-custom-genesis-multi-seed: runsim
	@echo "Running multi-seed custom genesis simulation..."
	@echo "By default, ${HOME}/.$(ARAGON_CHAIN_DAEMON_BINARY)/config/genesis.json will be used."
	@$(BINDIR)/runsim -Jobs=4 -SimAppPkg=$(SIMAPP) -ExitOnFail -Genesis=${HOME}/.$(ARAGON_CHAIN_DAEMON_BINARY)/config/genesis.json 400 5 TestFullAppSimulation

test-sim-multi-seed-long: runsim
	@echo "Running multi-seed application simulation. This may take awhile!"
	@$(BINDIR)/runsim -Jobs=4 -SimAppPkg=$(SIMAPP) -ExitOnFail 500 50 TestFullAppSimulation

test-sim-multi-seed-short: runsim
	@echo "Running multi-seed application simulation. This may take awhile!"
	@$(BINDIR)/runsim -Jobs=4 -SimAppPkg=$(SIMAPP) -ExitOnFail 50 10 TestFullAppSimulation

.PHONY: test test-unit test-race test-import test-rpc

.PHONY: test-sim-nondeterminism test-sim-custom-genesis-fast test-sim-import-export test-sim-after-import \
	test-sim-custom-genesis-multi-seed test-sim-multi-seed-long test-sim-multi-seed-short

###############################################################################
###                                Linting                                  ###
###############################################################################

lint:
	golangci-lint run --out-format=tab --issues-exit-code=0
	find . -name '*.go' -type f -not -path "./vendor*" -not -path "*.git*" | xargs gofmt -d -s

format:
	find . -name '*.go' -type f -not -path "./vendor*" -not -path "*.git*" -not -name '*.pb.go' | xargs gofmt -w -s
	find . -name '*.go' -type f -not -path "./vendor*" -not -path "*.git*" -not -name '*.pb.go' | xargs misspell -w
	find . -name '*.go' -type f -not -path "./vendor*" -not -path "*.git*" -not -name '*.pb.go' | xargs goimports -w -local github.com/tendermint
	find . -name '*.go' -type f -not -path "./vendor*" -not -path "*.git*" -not -name '*.pb.go' | xargs goimports -w -local github.com/ethereum/go-ethereum
	find . -name '*.go' -type f -not -path "./vendor*" -not -path "*.git*" -not -name '*.pb.go' | xargs goimports -w -local github.com/cosmos/cosmos-sdk
	find . -name '*.go' -type f -not -path "./vendor*" -not -path "*.git*" -not -name '*.pb.go' | xargs goimports -w -local github.com/cosmos/ethermint
	find . -name '*.go' -type f -not -path "./vendor*" -not -path "*.git*" -not -name '*.pb.go' | xargs goimports -w -local github.com/aragon/aragon-chain

.PHONY: lint format

#######################
###  Documentation  ###
#######################

# Start docs site at localhost:8080
docs-serve:
	@cd docs && \
	npm install && \
	npm run serve

# Build the site into docs/.vuepress/dist
docs-build:
	@cd docs && \
	npm install && \
	npm run build

godocs:
	@echo "--> Wait a few seconds and visit http://localhost:6060/pkg/github.com/aragon/aragon-chain"
	godoc -http=:6060

.PHONY: docs-serve docs-build

###############################################################################
###                                Localnet                                 ###
###############################################################################

build-docker-local-aragonchain:
	@$(MAKE) -C networks/local

# Run a 4-node testnet locally
localnet-start: localnet-stop
ifeq ($(OS),Windows_NT)
	mkdir build &
	@$(MAKE) docker-localnet

	IF not exist "build/node0/$(ARAGON_CHAIN_DAEMON_BINARY)/config/genesis.json" docker run --rm -v $(CURDIR)/build\aragonchain\Z aragonchaind/node "aragonchaind testnet --v 4 -o /aragonchain --starting-ip-address 192.168.10.2 --node-daemon-home=aragonchaind --node-cli-home=aragonchaincli --coin-denom=ara --keyring-backend=test --chain-id=aragonchain-1"
	docker-compose up -d
else
	mkdir -p ./build/
	@$(MAKE) docker-localnet

	if ! [ -f build/node0/$(ARAGON_CHAIN_DAEMON_BINARY)/config/genesis.json ]; then docker run --rm -v $(CURDIR)/build:/aragonchain:Z aragonchaind/node "aragonchaind testnet --v 4 -o /aragonchain --starting-ip-address 192.168.10.2 --node-daemon-home=aragonchaind --node-cli-home=aragonchaincli --coin-denom=ara --keyring-backend=test --chain-id=aragonchain-1"; fi
	docker-compose up -d
endif

localnet-stop:
	docker-compose down

# clean testnet
localnet-clean:
	docker-compose down
	sudo rm -rf build/*

 # reset testnet
localnet-unsafe-reset:
	docker-compose down
ifeq ($(OS),Windows_NT)
	@docker run --rm -v $(CURDIR)/build\aragonchain\Z aragonchaind/node "aragonchaind unsafe-reset-all --home=/aragonchain/node0/aragonchaind"
	@docker run --rm -v $(CURDIR)/build\aragonchain\Z aragonchaind/node "aragonchaind unsafe-reset-all --home=/aragonchain/node1/aragonchaind"
	@docker run --rm -v $(CURDIR)/build\aragonchain\Z aragonchaind/node "aragonchaind unsafe-reset-all --home=/aragonchain/node2/aragonchaind"
	@docker run --rm -v $(CURDIR)/build\aragonchain\Z aragonchaind/node "aragonchaind unsafe-reset-all --home=/aragonchain/node3/aragonchaind"
else
	@docker run --rm -v $(CURDIR)/build:/aragonchain:Z aragonchaind/node "aragonchaind unsafe-reset-all --home=/aragonchain/node0/aragonchaind"
	@docker run --rm -v $(CURDIR)/build:/aragonchain:Z aragonchaind/node "aragonchaind unsafe-reset-all --home=/aragonchain/node1/aragonchaind"
	@docker run --rm -v $(CURDIR)/build:/aragonchain:Z aragonchaind/node "aragonchaind unsafe-reset-all --home=/aragonchain/node2/aragonchaind"
	@docker run --rm -v $(CURDIR)/build:/aragonchain:Z aragonchaind/node "aragonchaind unsafe-reset-all --home=/aragonchain/node3/aragonchaind"
endif

.PHONY: build-docker-local-aragonchain localnet-start localnet-stop
