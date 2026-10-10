package main

import (
	"context"
	"os"
	"path/filepath"
	"testing"
	"time"

	"github.com/egoist/mygo"
	"github.com/oneo-me/onevra/plugins/webview"
)

func TestNativeCanvas(t *testing.T) {
	if os.Getenv("LOCUS_NATIVE_SMOKE") != "1" {
		t.Skip("requires a native desktop session")
	}
	root := t.TempDir()
	for p, body := range map[string]string{
		"template/index.md":          "---\ntitle: Template\n---\n",
		"template/canvas/index.html": "<main class=\"p-6\"><button id=\"counter\">Ready</button><div id=\"data\"></div></main>",
		"template/canvas/main.js":    `export async function main(data) { document.querySelector('#counter').textContent=data.message; const response=await fetch('locus://template/canvas/data.json'); document.querySelector('#data').textContent=(await response.json()).value; document.querySelector('#counter').onclick=()=>document.querySelector('#counter').textContent='Clicked'; }`,
		"template/canvas/data.json":  `{"value":"Local resource"}`,
		"caller/index.md":            "---\ntitle: Canvas 调用\nsummary: Canvas summary with comfortable spacing. 画布概要与下方内容分隔。\n---\n",
		"caller/canvas/index.json":   `{"template":"locus://template","data":{"message":"Template input"}}`,
		"ordinary/index.md":          "Ordinary Markdown",
	} {
		target := filepath.Join(root, filepath.FromSlash(p))
		if err := os.MkdirAll(filepath.Dir(target), 0700); err != nil {
			t.Fatal(err)
		}
		if err := os.WriteFile(target, []byte(body), 0600); err != nil {
			t.Fatal(err)
		}
	}
	a := &application{cursor: -1}
	mygo.RunOnMain(func() { a.start(root) })
	defer mygo.RunOnMain(func() { a.shell.Window().Destroy() })
	waitNative(t, func() bool { return !a.model.Loading })
	mygo.RunOnMain(func() { a.selectNote("caller") })
	waitNative(t, func() bool { return a.web != nil || a.canvasError != "" })
	var web *webview.WebView
	mygo.RunOnMain(func() {
		web = a.web
		if a.canvasError != "" {
			t.Error(a.canvasError)
		}
	})
	if web == nil {
		t.Fatal("canvas was not created")
	}
	var ready any
	deadline := time.Now().Add(10 * time.Second)
	for time.Now().Before(deadline) {
		ctx, cancel := context.WithTimeout(context.Background(), time.Second)
		ready, _ = web.Page().EvalContext(ctx, "document.documentElement.dataset.locusReady || document.documentElement.dataset.locusError || ''")
		cancel()
		if ready == "true" {
			break
		}
		time.Sleep(50 * time.Millisecond)
	}
	if ready != "true" {
		t.Fatalf("canvas failed to initialize: %v", ready)
	}
	result, err := web.Page().Eval("document.querySelector('#counter').textContent + ' / ' + document.querySelector('#data').textContent")
	if err != nil || result != "Template input / Local resource" {
		t.Fatalf("template/resources: %v, %v", result, err)
	}
	result, err = web.Page().Eval("document.querySelector('#counter').click(); return document.querySelector('#counter').textContent")
	if err != nil || result != "Clicked" {
		t.Fatalf("page interaction: %v, %v", result, err)
	}
	mygo.RunOnMain(func() { a.toggleSettings() })
	time.Sleep(100 * time.Millisecond)
	result, err = web.Page().Eval("document.querySelector('#counter').textContent")
	if err != nil || result != "Clicked" {
		t.Fatalf("settings lost page state: %v %v", result, err)
	}
	mygo.RunOnMain(func() { a.toggleSettings() })
	if dir := os.Getenv("LOCUS_SCREENSHOTS"); dir != "" {
		time.Sleep(150 * time.Millisecond)
		data, err := a.shell.Window().CapturePage()
		if err != nil {
			t.Fatal(err)
		}
		if err = os.MkdirAll(dir, 0700); err != nil {
			t.Fatal(err)
		}
		if err = os.WriteFile(filepath.Join(dir, "canvas.png"), data, 0600); err != nil {
			t.Fatal(err)
		}
	}
	mygo.RunOnMain(func() {
		a.selectNote("ordinary")
		if a.web != nil || a.session != nil {
			t.Error("leaving a canvas did not release browser")
		}
	})
}
