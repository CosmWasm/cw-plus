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
	"testing"

	"github.com/ChainSafe/gossamer/lib/runtime"
	"github.com/stretchr/testify/require"
)

// test used for ensuring runtime exec calls can me made concurrently
func TestConcurrentRuntimeCalls(t *testing.T) {
	instance := NewTestInstance(t, runtime.TEST_RUNTIME)

	// execute 2 concurrent calls to the runtime
	go func() {
		_, _ = instance.exec(runtime.CoreVersion, []byte{})
	}()
	go func() {
		_, _ = instance.exec(runtime.CoreVersion, []byte{})
	}()
}

func TestPointerSize(t *testing.T) {
	in := int64(8) + int64(32)<<32
	ptr, length := int64ToPointerAndSize(in)
	require.Equal(t, int32(8), ptr)
	require.Equal(t, int32(32), length)
	res := pointerAndSizeToInt64(ptr, length)
	require.Equal(t, in, res)
}
