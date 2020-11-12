package state

import (
	"sort"
	"testing"

	"github.com/ChainSafe/gossamer/lib/common"
	"github.com/ChainSafe/gossamer/lib/transaction"

	"github.com/stretchr/testify/require"
)

func TestTransactionState_Pending(t *testing.T) {
	ts := NewTransactionState()

	txs := []*transaction.ValidTransaction{
		{
			Extrinsic: []byte("a"),
			Validity:  &transaction.Validity{Priority: 1},
		},
		{
			Extrinsic: []byte("b"),
			Validity:  &transaction.Validity{Priority: 4},
		},
		{
			Extrinsic: []byte("c"),
			Validity:  &transaction.Validity{Priority: 2},
		},
		{
			Extrinsic: []byte("d"),
			Validity:  &transaction.Validity{Priority: 17},
		},
		{
			Extrinsic: []byte("e"),
			Validity:  &transaction.Validity{Priority: 2},
		},
	}

	hashes := make([]common.Hash, len(txs))
	for i, tx := range txs {
		h := ts.AddToPool(tx)
		hashes[i] = h
	}

	pendingPool := ts.PendingInPool()

	sort.Slice(pendingPool, func(i, j int) bool {
		return pendingPool[i].Extrinsic[0] < pendingPool[j].Extrinsic[0]
	})
	require.Equal(t, pendingPool, txs)

	pending := ts.Pending()
	sort.Slice(pending, func(i, j int) bool {
		return pending[i].Extrinsic[0] < pending[j].Extrinsic[0]
	})
	require.Equal(t, pending, txs)

	// queue should be empty
	head := ts.Peek()
	require.Nil(t, head)
}
