package library

import (
	"net/url"
	"path"
	"sort"
	"strings"

	"github.com/yuin/goldmark"
	"github.com/yuin/goldmark/ast"
	"github.com/yuin/goldmark/extension"
	"github.com/yuin/goldmark/text"
	"github.com/yuin/goldmark/util"
)

// Relation is a directed link from one note to another, discovered in the
// link destination of a Markdown link in the source note's body.
type Relation struct {
	Source, Target string
}

// ResolveNoteLink maps a Markdown link destination to a library-relative note
// path. Locus addresses resolve directly; other destinations must be relative,
// without a scheme or host, and never leave the library root. A destination
// ending in index.md names its containing note directory.
func ResolveNoteLink(note, target string) (string, bool) {
	target = strings.TrimSpace(target)
	if target == "" {
		return "", false
	}
	if strings.HasPrefix(target, "locus://") {
		p, err := ParseAddress(target)
		if err != nil {
			return "", false
		}
		return p, true
	}
	u, err := url.Parse(target)
	if err != nil || u.Scheme != "" || u.Host != "" {
		return "", false
	}
	decoded, err := url.PathUnescape(u.EscapedPath())
	if err != nil || decoded == "" {
		return "", false
	}
	p := strings.TrimSuffix(path.Join(note, decoded), "/index.md")
	if !validNote(p) {
		return "", false
	}
	return p, true
}

// NoteLinks returns the sorted, unique note paths a body links to. Only
// Markdown links count; images, autolinks and other destinations are ignored.
func NoteLinks(note, body string) []string {
	data := []byte(body)
	root := goldmark.New(goldmark.WithExtensions(extension.GFM)).Parser().Parse(text.NewReader(data))
	seen := map[string]bool{}
	var links []string
	_ = ast.Walk(root, func(n ast.Node, entering bool) (ast.WalkStatus, error) {
		if !entering {
			return ast.WalkContinue, nil
		}
		link, ok := n.(*ast.Link)
		if !ok {
			return ast.WalkContinue, nil
		}
		target, ok := ResolveNoteLink(note, decodeDestination(link.Destination))
		if ok && target != note && !seen[target] {
			seen[target] = true
			links = append(links, target)
		}
		return ast.WalkContinue, nil
	})
	sort.Strings(links)
	return links
}

// Relations reads every note body in the given tree and returns the directed
// links between existing notes. A note whose body cannot be read is skipped so
// one unreadable file does not hide the rest of the graph.
func (l *Library) Relations(notes []*Note) []Relation {
	known := map[string]bool{}
	var mark func([]*Note)
	mark = func(list []*Note) {
		for _, n := range list {
			known[n.Path] = true
			mark(n.Children)
		}
	}
	mark(notes)
	seen := map[Relation]bool{}
	var relations []Relation
	var walk func([]*Note)
	walk = func(list []*Note) {
		for _, n := range list {
			if n.HasBody {
				if document, err := l.Read(n.Path); err == nil {
					for _, target := range NoteLinks(n.Path, document.Body) {
						if !known[target] {
							continue
						}
						relation := Relation{Source: n.Path, Target: target}
						if !seen[relation] {
							seen[relation] = true
							relations = append(relations, relation)
						}
					}
				}
			}
			walk(n.Children)
		}
	}
	walk(notes)
	sort.Slice(relations, func(i, j int) bool {
		if relations[i].Source != relations[j].Source {
			return relations[i].Source < relations[j].Source
		}
		return relations[i].Target < relations[j].Target
	})
	return relations
}

func decodeDestination(value []byte) string {
	return string(util.ResolveEntityNames(util.ResolveNumericReferences(util.UnescapePunctuations(value))))
}
