package types

import (
	sdk "github.com/cosmos/cosmos-sdk/types"
)

const (
	// ARA defines the default coin denomination used in Aragon Chain in:
	//
	// - Staking parameters: denomination used as stake in the dPoS chain
	// - Mint parameters: denomination minted due to fee distribution rewards
	// - Governance parameters: denomination used for spam prevention in proposal deposits
	// - Crisis parameters: constant fee denomination used for spam prevention to check broken invariant
	// - EVM parameters: denomination used for running EVM state transitions in Ethermint.
	ARA string = "ara"
)

// NewAraCoin is a utility function that returns an "ara" coin with the given sdk.Int amount.
// The function will panic if the provided amount is negative.
func NewAraCoin(amount sdk.Int) sdk.Coin {
	return sdk.NewCoin(ARA, amount)
}

// NewAraCoinInt64 is a utility function that returns an "ara" coin with the given int64 amount.
// The function will panic if the provided amount is negative.
func NewAraCoinInt64(amount int64) sdk.Coin {
	return sdk.NewInt64Coin(ARA, amount)
}
