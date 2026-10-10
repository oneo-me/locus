package library

import (
	"context"
	"os"
	"path/filepath"
	"testing"
	"time"
)

func TestRecursiveWatchAndCancellation(t *testing.T) {
	root := t.TempDir()
	put(t, root, "note/index.md", "Before")
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()
	changes := make(chan struct{}, 20)
	failures := make(chan error, 1)
	done := make(chan struct{})
	go func() {
		defer close(done)
		Watch(ctx, root, func() { changes <- struct{}{} }, func(err error) { failures <- err })
	}()
	wait := func() {
		t.Helper()
		select {
		case <-changes:
		case err := <-failures:
			t.Fatal(err)
		case <-time.After(5 * time.Second):
			t.Fatal("watch notification timed out")
		}
	}
	wait()
	if err := os.WriteFile(filepath.Join(root, "note/index.md"), []byte("After"), 0600); err != nil {
		t.Fatal(err)
	}
	wait()
	put(t, root, "note/child/index.md", "Child")
	wait()
	cancel()
	select {
	case <-done:
	case <-time.After(time.Second):
		t.Fatal("watch did not stop")
	}
}
