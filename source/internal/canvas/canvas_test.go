package canvas

import (
	"archive/zip"
	"io"
	"net/http/httptest"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func put(t *testing.T, root, p, body string) {
	t.Helper()
	p = filepath.Join(root, filepath.FromSlash(p))
	if err := os.MkdirAll(filepath.Dir(p), 0700); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(p, []byte(body), 0600); err != nil {
		t.Fatal(err)
	}
}
func fixture(t *testing.T) string {
	root := t.TempDir()
	for p, body := range map[string]string{"template/index.md": "---\ntitle: Template\n---\nBody\n", "template/canvas/index.html": "<p id=\"message\"></p>", "template/canvas/main.js": "export async function main(data) {}", "template/canvas/data.json": "{\"answer\":42}", "caller/index.md": "Caller", "caller/canvas/index.json": "{\"template\":\"locus://template\",\"data\":{\"text\":\"</script><script>bad()</script>\"}}", "unrelated/index.md": "Private"} {
		put(t, root, p, body)
	}
	return root
}
func request(router *Router, url string) *httptest.ResponseRecorder {
	w := httptest.NewRecorder()
	router.ServeHTTP(w, httptest.NewRequest("GET", url, nil))
	return w
}
func TestTemplateResourcesAndExport(t *testing.T) {
	var r Router
	s, err := r.Open(fixture(t), "caller", "Test")
	if err != nil {
		t.Fatal(err)
	}
	defer r.CloseAll()
	page := request(&r, s.URL())
	if page.Code != 200 || !strings.Contains(page.Body.String(), "files/template/canvas/") {
		t.Fatalf("page: %d", page.Code)
	}
	if strings.Contains(page.Body.String(), "</script><script>bad()") {
		t.Fatal("template JSON escaped its script")
	}
	if !strings.Contains(page.Header().Get("Content-Security-Policy"), "frame-src 'none'") {
		t.Fatal("missing CSP")
	}
	for _, p := range []string{"template/canvas/main.js", "template/canvas/data.json", "template/index.md?part=body"} {
		if got := request(&r, s.prefix()+"files/"+p); got.Code != 200 {
			t.Fatalf("resource %s: %d", p, got.Code)
		}
	}
	body := request(&r, s.prefix()+"files/template/index.md?part=body")
	if body.Body.String() != "Body\n" {
		t.Fatal(body.Body.String())
	}
	target, err := s.Export(t.TempDir())
	if err != nil {
		t.Fatal(err)
	}
	archive, err := zip.OpenReader(target)
	if err != nil {
		t.Fatal(err)
	}
	defer archive.Close()
	found := map[string]bool{}
	for _, file := range archive.File {
		found[file.Name] = true
		if strings.Contains(file.Name, "unrelated") {
			t.Fatal("export included unrelated notes")
		}
		if file.Name == "canvas.html" {
			f, _ := file.Open()
			b, _ := io.ReadAll(f)
			f.Close()
			if strings.Contains(string(b), s.token) {
				t.Fatal("export retained ephemeral origin")
			}
		}
	}
	for _, name := range []string{"canvas.html", "files/template/canvas/main.js", "files/template/canvas/data.json", "files/template/index.md.locus-body", "TAILWIND-LICENSE.txt"} {
		if !found[name] {
			t.Errorf("missing %s", name)
		}
	}
}
func TestBoundaryAndSessionRelease(t *testing.T) {
	var r Router
	root := fixture(t)
	s, err := r.Open(root, "template", "Test")
	if err != nil {
		t.Fatal(err)
	}
	for _, p := range []string{"../unrelated/index.md", "template/canvas/%2e%2e/index.md", "template%2fcanvas/main.js", ".hidden/index.md", "template/canvas/node_modules/x.js", "template/index.md?part=invalid", "unrelated/secrets.txt"} {
		if got := request(&r, s.prefix()+"files/"+p); got.Code == 200 {
			t.Errorf("accepted %s", p)
		}
	}
	outside := t.TempDir()
	put(t, outside, "secret.txt", "Secret")
	if err := os.Symlink(filepath.Join(outside, "secret.txt"), filepath.Join(root, "template/canvas/linked")); err == nil {
		if got := request(&r, s.prefix()+"files/template/canvas/linked"); got.Code == 200 {
			t.Fatal("followed symlink")
		}
	}
	r.Close(s)
	if got := request(&r, s.URL()); got.Code != 404 {
		t.Fatal("released page accessible")
	}
	r.CloseAll()
	if _, err := r.Open(root, "template", "Again"); err == nil {
		t.Fatal("closed router accepted a page")
	}
}
func TestInvalidCanvas(t *testing.T) {
	for _, config := range []string{`{"template":"locus://caller","data":null}`, `{"template":"locus://template"}`, `{"template":"locus://template","data":null,"extra":1}`, `{"template":"locus://../escape","data":null}`} {
		root := fixture(t)
		put(t, root, "caller/canvas/index.json", config)
		var r Router
		if _, err := r.Open(root, "caller", "Test"); err == nil {
			t.Fatalf("accepted %s", config)
		}
	}
	root := fixture(t)
	put(t, root, "caller/canvas/index.html", "conflict")
	var r Router
	if _, err := r.Open(root, "caller", "Test"); err == nil {
		t.Fatal("accepted dual entry points")
	}
}
