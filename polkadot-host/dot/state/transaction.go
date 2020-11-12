package state

import (
	"github.com/ChainSafe/gossamer/dot/types"
	"github.com/ChainSafe/gossamer/lib/common"
	"github.com/ChainSafe/gossamer/lib/transaction"
)

// TransactionState represents the queue of transactions
type TransactionState struct {
	queue *transaction.PriorityQueue
	pool  *transaction.Pool
}

// NewTransactionState returns a new TransactionState
func NewTransactionState() *TransactionState {
	return &TransactionState{
		queue: transaction.NewPriorityQueue(),
		pool:  transaction.NewPool(),
	}
}

// Push pushes a transaction to the queue, ordered by priority
func (s *TransactionState) Push(vt *transaction.ValidTransaction) (common.Hash, error) {
	return s.queue.Push(vt)
}

// Pop removes and returns the head of the queue
func (s *TransactionState) Pop() *transaction.ValidTransaction {
	return s.queue.Pop()
}

// Peek returns the head of the queue without removing it
func (s *TransactionState) Peek() *transaction.ValidTransaction {
	return s.queue.Peek()
}

// Pending returns the current transactions in the queue and pool
func (s *TransactionState) Pending() []*transaction.ValidTransaction {
	return append(s.queue.Pending(), s.pool.Transactions()...)
}

// PendingInPool returns the current transactions in the pool
func (s *TransactionState) PendingInPool() []*transaction.ValidTransaction {
	return s.pool.Transactions()
}

// RemoveExtrinsic removes an extrinsic from the queue and pool
func (s *TransactionState) RemoveExtrinsic(ext types.Extrinsic) {
	s.pool.Remove(ext.Hash())
	s.queue.RemoveExtrinsic(ext)
}

// RemoveExtrinsicFromPool removes an extrinsic from the pool
func (s *TransactionState) RemoveExtrinsicFromPool(ext types.Extrinsic) {
	s.pool.Remove(ext.Hash())
}

// AddToPool adds a transaction to the pool
func (s *TransactionState) AddToPool(vt *transaction.ValidTransaction) common.Hash {
	return s.pool.Insert(vt)
}
