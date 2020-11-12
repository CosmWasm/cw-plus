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

package main

import (
	"fmt"
	"os"
	"path/filepath"
	"reflect"
	"unicode"

	"github.com/naoina/toml"

	ctoml "github.com/ChainSafe/gossamer/dot/config/toml"
)

// loadConfig loads the values from the toml configuration file into the provided configuration
func loadConfig(cfg *ctoml.Config, fp string) error {
	fp, err := filepath.Abs(fp)
	if err != nil {
		logger.Error("failed to create absolute path for toml configuration file", "error", err)
		return err
	}

	file, err := os.Open(filepath.Clean(fp))
	if err != nil {
		logger.Error("failed to open toml configuration file", "error", err)
		return err
	}

	var tomlSettings = toml.Config{
		NormFieldName: func(rt reflect.Type, key string) string {
			return key
		},
		FieldToKey: func(rt reflect.Type, field string) string {
			return field
		},
		MissingField: func(rt reflect.Type, field string) error {
			link := ""
			if unicode.IsUpper(rune(rt.Name()[0])) && rt.PkgPath() != "main" {
				link = fmt.Sprintf(", see https://godoc.org/%s#%s for available fields", rt.PkgPath(), rt.Name())
			}
			return fmt.Errorf("field '%s' is not defined in %s%s", field, rt.String(), link)
		},
	}

	if err = tomlSettings.NewDecoder(file).Decode(&cfg); err != nil {
		logger.Error("failed to decode configuration", "error", err)
		return err
	}

	return nil
}

// exportConfig exports a dot configuration to a toml configuration file
func exportConfig(cfg *ctoml.Config, fp string) *os.File {
	var (
		newFile *os.File
		err     error
		raw     []byte
	)

	if raw, err = toml.Marshal(*cfg); err != nil {
		logger.Error("failed to marshal configuration", "error", err)
		os.Exit(1)
	}

	newFile, err = os.Create(filepath.Clean(fp))
	if err != nil {
		logger.Error("failed to create configuration file", "error", err)
		os.Exit(1)
	}

	_, err = newFile.Write(raw)
	if err != nil {
		logger.Error("failed to write to configuration file", "error", err)
		os.Exit(1)
	}

	if err := newFile.Close(); err != nil {
		logger.Error("failed to close configuration file", "error", err)
		os.Exit(1)
	}

	return newFile
}
