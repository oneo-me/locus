//go:build !darwin

package main

import (
	"testing"

	"github.com/egoist/mygo"
)

// Other platforms keep visible native review until hidden frame delivery has
// been verified against their window systems.
func nativeBackground() bool                         { return false }
func startNativeFrames() func()                      { return func() {} }
func checkNativeBackground(*testing.T, *mygo.Window) {}
