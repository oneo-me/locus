package main

import (
	"os"
	"testing"
	"time"

	"github.com/ebitengine/purego/objc"
	"github.com/egoist/mygo"
)

// Native tests run hidden unless LOCUS_NATIVE_FOREGROUND=1 asks for visible
// windows, focus and a Dock icon.
func nativeBackground() bool { return os.Getenv("LOCUS_NATIVE_FOREGROUND") != "1" }

// AppKit suspends display-link ticks for hidden windows. Ask their view trees
// to display on the main thread so MyGo consumes input and builds fresh
// captures. This belongs to the test harness, not the app lifecycle.
func startNativeFrames() func() {
	if !nativeBackground() {
		return func() {}
	}
	stop, done := make(chan struct{}), make(chan struct{})
	go func() {
		defer close(done)
		ticker := time.NewTicker(time.Second / 30)
		defer ticker.Stop()
		for {
			select {
			case <-stop:
				return
			case <-ticker.C:
				mygo.RunOnMain(func() {
					for _, w := range mygo.Windows() {
						if handle := w.NativeHandle(); handle != 0 && !w.IsVisible() {
							displayNativeContent(objc.ID(handle).Send(objc.RegisterName("contentView")))
						}
					}
				})
			}
		}
	}()
	return func() { close(stop); <-done }
}

func displayNativeContent(view objc.ID) {
	if view.Send(objc.RegisterName("isKindOfClass:"), objc.GetClass("MyGoSurfaceView")) != 0 {
		view.Send(objc.RegisterName("setNeedsDisplay:"), true)
		view.Send(objc.RegisterName("display"))
		return
	}
	children := view.Send(objc.RegisterName("subviews"))
	for i, n := uint(0), uint(children.Send(objc.RegisterName("count"))); i < n; i++ {
		displayNativeContent(children.Send(objc.RegisterName("objectAtIndex:"), i))
	}
}

// checkNativeBackground reports a hidden test window that became visible or
// focused, or an activated test app. Call it on the main thread.
func checkNativeBackground(t *testing.T, w *mygo.Window) {
	t.Helper()
	if !nativeBackground() {
		return
	}
	app := objc.ID(objc.GetClass("NSApplication")).Send(objc.RegisterName("sharedApplication"))
	if w.IsVisible() || w.IsFocused() || objc.Send[bool](app, objc.RegisterName("isActive")) {
		t.Error("background native test showed a window or activated the app")
	}
}
