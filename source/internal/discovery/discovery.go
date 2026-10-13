// Package discovery implements the read-only MCP file-discovery service.
package discovery

import (
	"context"
	"errors"
	"fmt"
	"net"
	"net/http"
	"path/filepath"
	"strconv"
	"strings"
	"time"

	"github.com/modelcontextprotocol/go-sdk/mcp"
	"github.com/oneo-me/locus/internal/library"
)

type Article struct {
	Path      string `json:"path"`
	Title     string `json:"title"`
	Summary   string `json:"summary,omitempty"`
	File      string `json:"file"`
	Directory string `json:"directory"`
	Excerpt   string `json:"excerpt,omitempty"`
}
type Page struct {
	Articles   []Article `json:"articles"`
	NextCursor *string   `json:"nextCursor"`
}
type ListInput struct {
	Parent string `json:"parent,omitempty"`
	Cursor string `json:"cursor,omitempty"`
	Limit  int    `json:"limit,omitempty"`
}
type SearchInput struct {
	Query string `json:"query"`
	Limit int    `json:"limit,omitempty"`
}
type ResolveInput struct {
	Path string `json:"path"`
}

func normalize(p string) (string, error) {
	if p == "" || p == "locus://" {
		return "", nil
	}
	p = strings.TrimSuffix(p, "/index.md")
	if !strings.HasPrefix(p, "locus://") {
		p = library.Address(p)
	}
	return library.ParseAddress(p)
}
func article(root string, n *library.Note) Article {
	dir := filepath.Join(root, filepath.FromSlash(n.Path))
	return Article{Path: n.Path, Title: n.Title, Summary: n.Summary, File: filepath.Join(dir, "index.md"), Directory: dir}
}
func scan(root string) (*library.Library, []*library.Note, error) {
	if root == "" {
		return nil, nil, errors.New("select a library first")
	}
	lib, err := library.Open(root)
	if err != nil {
		return nil, nil, err
	}
	notes, err := lib.Scan()
	if err != nil {
		lib.Close()
		return nil, nil, err
	}
	return lib, notes, nil
}
func List(root string, in ListInput) (Page, error) {
	lib, notes, err := scan(root)
	if err != nil {
		return Page{}, err
	}
	defer lib.Close()
	parent, err := normalize(in.Parent)
	if err != nil {
		return Page{}, err
	}
	if parent != "" {
		n := library.Find(notes, parent)
		if n == nil {
			return Page{}, errors.New("parent not found")
		}
		notes = n.Children
	}
	start := 0
	if in.Cursor != "" {
		start, err = strconv.Atoi(in.Cursor)
		if err != nil || start < 0 || start > len(notes) {
			return Page{}, errors.New("invalid cursor")
		}
	}
	limit := in.Limit
	if limit == 0 {
		limit = 50
	}
	if limit < 1 || limit > 200 {
		return Page{}, errors.New("limit must be 1..200")
	}
	end := min(start+limit, len(notes))
	result := Page{Articles: []Article{}}
	for _, n := range notes[start:end] {
		a := article(lib.Path, n)
		if !n.HasBody {
			a.File = ""
		}
		result.Articles = append(result.Articles, a)
	}
	if end < len(notes) {
		cursor := strconv.Itoa(end)
		result.NextCursor = &cursor
	}
	return result, nil
}
func Resolve(root string, in ResolveInput) (Article, error) {
	p, err := normalize(in.Path)
	if err != nil || p == "" {
		return Article{}, errors.New("invalid note path")
	}
	lib, notes, err := scan(root)
	if err != nil {
		return Article{}, err
	}
	defer lib.Close()
	n := library.Find(notes, p)
	if n == nil || !n.HasBody {
		return Article{}, errors.New("note not found")
	}
	return article(lib.Path, n), nil
}
func Search(ctx context.Context, root string, in SearchInput) (Page, error) {
	lib, notes, err := scan(root)
	if err != nil {
		return Page{}, err
	}
	defer lib.Close()
	limit := in.Limit
	if limit == 0 {
		limit = 20
	}
	if limit < 1 || limit > 100 {
		return Page{}, errors.New("limit must be 1..100")
	}
	query := strings.ToLower(strings.TrimSpace(in.Query))
	if query == "" {
		return Page{}, errors.New("query is empty")
	}
	result := Page{Articles: []Article{}}
	var walk func([]*library.Note) error
	walk = func(notes []*library.Note) error {
		for _, n := range notes {
			if err := ctx.Err(); err != nil {
				return err
			}
			if len(result.Articles) >= limit {
				return nil
			}
			if n.HasBody {
				doc, err := lib.Read(n.Path)
				if err != nil {
					return err
				}
				if strings.Contains(strings.ToLower(n.Path+"\n"+n.Title+"\n"+n.Summary+"\n"+doc.Body), query) {
					a := article(lib.Path, n)
					runes := []rune(doc.Body)
					a.Excerpt = string(runes[:min(240, len(runes))])
					result.Articles = append(result.Articles, a)
				}
			}
			if err := walk(n.Children); err != nil {
				return err
			}
		}
		return nil
	}
	err = walk(notes)
	return result, err
}
func Handler(root string) http.Handler {
	server := mcp.NewServer(&mcp.Implementation{Name: "Locus", Version: "0.2026.1"}, &mcp.ServerOptions{Instructions: "Read-only local note discovery. Read and edit returned files with your own filesystem tools, following the installed Locus skill."})
	mcp.AddTool(server, &mcp.Tool{Name: "list_articles", Description: "List direct child notes and organizing directories."}, func(ctx context.Context, r *mcp.CallToolRequest, in ListInput) (*mcp.CallToolResult, Page, error) {
		v, e := List(root, in)
		return nil, v, e
	})
	mcp.AddTool(server, &mcp.Tool{Name: "resolve_article", Description: "Resolve a note path or locus:// address to real files."}, func(ctx context.Context, r *mcp.CallToolRequest, in ResolveInput) (*mcp.CallToolResult, Article, error) {
		v, e := Resolve(root, in)
		return nil, v, e
	})
	mcp.AddTool(server, &mcp.Tool{Name: "search_articles", Description: "Search visible note paths, titles, summaries and Markdown bodies."}, func(ctx context.Context, r *mcp.CallToolRequest, in SearchInput) (*mcp.CallToolResult, Page, error) {
		v, e := Search(ctx, root, in)
		return nil, v, e
	})
	transport := mcp.NewStreamableHTTPHandler(func(*http.Request) *mcp.Server { return server }, &mcp.StreamableHTTPOptions{Stateless: true, JSONResponse: true, MaxRequestBodyBytes: 1 << 20})
	protection := http.NewCrossOriginProtection()
	return protection.Handler(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/mcp" {
			http.NotFound(w, r)
			return
		}
		host := r.Host
		if h, _, err := net.SplitHostPort(host); err == nil {
			host = h
		}
		host = strings.Trim(host, "[]")
		if host != "localhost" && !(net.ParseIP(host) != nil && net.ParseIP(host).IsLoopback()) {
			http.Error(w, "Host rejected", 403)
			return
		}
		transport.ServeHTTP(w, r)
	}))
}

type Service struct {
	server  *http.Server
	done    chan error
	Address string
}

func Start(root string, port int) (*Service, error) {
	if root == "" {
		return nil, errors.New("select a library before enabling MCP")
	}
	if port < 1 || port > 65535 {
		return nil, errors.New("invalid MCP port")
	}
	ln, err := net.Listen("tcp", net.JoinHostPort("127.0.0.1", strconv.Itoa(port)))
	if err != nil {
		return nil, err
	}
	s := &Service{server: &http.Server{Handler: Handler(root), ReadHeaderTimeout: 5 * time.Second, IdleTimeout: 60 * time.Second, WriteTimeout: 60 * time.Second}, done: make(chan error, 1), Address: ln.Addr().String()}
	go func() { s.done <- s.server.Serve(ln) }()
	return s, nil
}
func (s *Service) Close() error {
	if s == nil {
		return nil
	}
	if err := s.server.Close(); err != nil {
		return err
	}
	err := <-s.done
	if errors.Is(err, http.ErrServerClosed) {
		return nil
	}
	return fmt.Errorf("MCP: %w", err)
}
