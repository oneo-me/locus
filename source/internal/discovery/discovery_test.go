package discovery

import (
	"context"
	"net/http/httptest"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"

	"github.com/modelcontextprotocol/go-sdk/mcp"
)

func fixture(t *testing.T) string {
	t.Helper()
	root := t.TempDir()
	for name, body := range map[string]string{"a": "---\ntitle: 中文 Title\n---\nSearch needle", "b": "Second", "hidden": "---\narchived: true\n---\nneedle"} {
		p := filepath.Join(root, name)
		if err := os.Mkdir(p, 0700); err != nil {
			t.Fatal(err)
		}
		if err := os.WriteFile(filepath.Join(p, "index.md"), []byte(body), 0600); err != nil {
			t.Fatal(err)
		}
	}
	return root
}
func TestDiscoveryContracts(t *testing.T) {
	root := fixture(t)
	result, err := List(root, ListInput{Limit: 1})
	if err != nil || len(result.Articles) != 1 || result.NextCursor == nil {
		t.Fatalf("page=%+v err=%v", result, err)
	}
	next, err := List(root, ListInput{Limit: 1, Cursor: *result.NextCursor})
	if err != nil || len(next.Articles) != 1 || next.NextCursor != nil {
		t.Fatalf("next=%+v err=%v", next, err)
	}
	item, err := Resolve(root, ResolveInput{Path: "locus://a/index.md"})
	if err != nil || item.Title != "中文 Title" || !filepath.IsAbs(item.File) {
		t.Fatalf("resolve=%+v %v", item, err)
	}
	matches, err := Search(context.Background(), root, SearchInput{Query: "NEEDLE"})
	if err != nil || len(matches.Articles) != 1 || matches.Articles[0].Path != "a" {
		t.Fatalf("search=%+v %v", matches, err)
	}
	for _, p := range []string{"../a", "hidden", "locus://a%2Findex.md", ""} {
		if _, err := Resolve(root, ResolveInput{Path: p}); err == nil {
			t.Errorf("accepted %q", p)
		}
	}
	if _, err := List(root, ListInput{Cursor: "-1"}); err == nil {
		t.Fatal("negative cursor accepted")
	}
}
func TestStreamableHTTPAndHostProtection(t *testing.T) {
	handler := Handler(fixture(t))
	server := httptest.NewServer(handler)
	defer server.Close()
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()
	client := mcp.NewClient(&mcp.Implementation{Name: "test", Version: "1"}, nil)
	session, err := client.Connect(ctx, &mcp.StreamableClientTransport{Endpoint: server.URL + "/mcp"}, nil)
	if err != nil {
		t.Fatal(err)
	}
	defer session.Close()
	tools, err := session.ListTools(ctx, nil)
	if err != nil || len(tools.Tools) != 3 {
		t.Fatalf("tools=%+v %v", tools, err)
	}
	result, err := session.CallTool(ctx, &mcp.CallToolParams{Name: "resolve_article", Arguments: map[string]any{"path": "a"}})
	if err != nil || result.IsError {
		t.Fatalf("call=%+v %v", result, err)
	}
	req := httptest.NewRequest("POST", "http://evil.invalid/mcp", strings.NewReader(`{}`))
	response := httptest.NewRecorder()
	handler.ServeHTTP(response, req)
	if response.Code != 403 {
		t.Fatalf("Host accepted: %d", response.Code)
	}
	req = httptest.NewRequest("POST", "http://127.0.0.1/mcp", strings.NewReader(`{}`))
	req.Header.Set("Origin", "https://evil.invalid")
	response = httptest.NewRecorder()
	handler.ServeHTTP(response, req)
	if response.Code != 403 {
		t.Fatalf("Origin accepted: %d", response.Code)
	}
}
