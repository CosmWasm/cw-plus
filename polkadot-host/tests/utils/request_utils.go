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
	"bytes"
	"encoding/json"
	"errors"
	"fmt"
	"io/ioutil"
	"net/http"
	"testing"
	"time"

	"github.com/stretchr/testify/require"
)

// PostRPC utils for sending payload to endpoint and getting []byte back
func PostRPC(method, host, params string) ([]byte, error) {
	data := []byte(`{"jsonrpc":"2.0","method":"` + method + `","params":` + params + `,"id":1}`)
	buf := &bytes.Buffer{}
	_, err := buf.Write(data)
	if err != nil {
		return nil, err
	}

	r, err := http.NewRequest("POST", host, buf)
	if err != nil {
		return nil, err
	}

	r.Header.Set("Content-Type", ContentTypeJSON)
	r.Header.Set("Accept", ContentTypeJSON)

	resp, err := httpClient.Do(r)
	if err != nil {
		return nil, err
	} else if resp.StatusCode != http.StatusOK {
		return nil, fmt.Errorf("status code not OK")
	}

	defer func() {
		_ = resp.Body.Close()
	}()

	respBody, err := ioutil.ReadAll(resp.Body)

	return respBody, err

}

// PostRPCWithRetry is a wrapper around `PostRPC` that calls it `retry` number of times.
func PostRPCWithRetry(method, host, params string, retry int) ([]byte, error) {
	count := 0
	for {
		resp, err := PostRPC(method, host, params)
		if err == nil || count >= retry {
			return resp, err
		}
		time.Sleep(200 * time.Millisecond)
		count++
	}
}

// DecodeRPC will decode []body into target interface
func DecodeRPC(t *testing.T, body []byte, target interface{}) error {
	decoder := json.NewDecoder(bytes.NewReader(body))
	decoder.DisallowUnknownFields()

	var response ServerResponse
	err := decoder.Decode(&response)
	require.Nil(t, err, string(body))
	require.Equal(t, response.Version, "2.0")

	if response.Error != nil {
		return errors.New(response.Error.Message)
	}

	decoder = json.NewDecoder(bytes.NewReader(response.Result))
	decoder.DisallowUnknownFields()

	err = decoder.Decode(target)
	require.Nil(t, err, string(body))
	return nil
}

// DecodeWebsocket will decode body into target interface
func DecodeWebsocket(t *testing.T, body []byte, target interface{}) error {
	decoder := json.NewDecoder(bytes.NewReader(body))
	decoder.DisallowUnknownFields()

	var response WebsocketResponse
	err := decoder.Decode(&response)
	require.Nil(t, err, string(body))
	require.Equal(t, response.Version, "2.0")

	if response.Error != nil {
		return errors.New(response.Error.Message)
	}

	if response.Result != nil {
		decoder = json.NewDecoder(bytes.NewReader(response.Result))
	} else {
		decoder = json.NewDecoder(bytes.NewReader(response.Params))
	}

	decoder.DisallowUnknownFields()

	err = decoder.Decode(target)
	require.Nil(t, err, string(body))
	return nil
}

// DecodeRPC_NT will decode []body into target interface (NT is Not Test testing required)
func DecodeRPC_NT(body []byte, target interface{}) error {
	decoder := json.NewDecoder(bytes.NewReader(body))
	decoder.DisallowUnknownFields()

	var response ServerResponse
	err := decoder.Decode(&response)
	if err != nil {
		return err
	}

	if response.Error != nil {
		return errors.New(response.Error.Message)
	}

	decoder = json.NewDecoder(bytes.NewReader(response.Result))
	decoder.DisallowUnknownFields()

	err = decoder.Decode(target)
	return err
}

// NewEndpoint will create a new endpoint string based on utils.HOSTNAME and port
func NewEndpoint(port string) string {
	return "http://" + HOSTNAME + ":" + port
}
