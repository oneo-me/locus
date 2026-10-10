package library

import (
	"context"
	"io/fs"
	"path/filepath"
	"strings"
	"time"

	"github.com/fsnotify/fsnotify"
)

// Watch recursively subscribes, coalescing notifications. It never polls or
// claims freshness after a watcher error. The owner cancels on library switch.
func Watch(ctx context.Context, root string, changed func(), failed func(error)) {
	w, err := fsnotify.NewWatcher()
	if err != nil {
		failed(err)
		return
	}
	defer w.Close()
	add := func() error {
		return filepath.WalkDir(root, func(p string, d fs.DirEntry, err error) error {
			if err != nil {
				return err
			}
			if d.Type()&fs.ModeSymlink != 0 {
				return nil
			}
			if !d.IsDir() {
				return nil
			}
			if p != root && (strings.HasPrefix(d.Name(), ".") || d.Name() == "node_modules") {
				return filepath.SkipDir
			}
			return w.Add(p)
		})
	}
	if err = add(); err != nil {
		failed(err)
		return
	}
	changed() // Reconcile changes occurring between the initial scan and watches.
	timer := time.NewTimer(time.Hour)
	timer.Stop()
	defer timer.Stop()
	var tick <-chan time.Time
	for {
		select {
		case <-ctx.Done():
			return
		case err, ok := <-w.Errors:
			if !ok {
				return
			}
			failed(err)
			return
		case e, ok := <-w.Events:
			if !ok {
				return
			}
			if e.Op&fsnotify.Create != 0 {
				if err := add(); err != nil {
					failed(err)
					return
				}
			}
			timer.Reset(150 * time.Millisecond)
			tick = timer.C
		case <-tick:
			tick = nil
			changed()
		}
	}
}
