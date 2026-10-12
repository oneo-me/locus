package main

import (
	"fmt"
	"path/filepath"
	"strings"

	"github.com/egoist/mygo"
	"github.com/egoist/mygo/ui"
	"github.com/oneo-me/locus/internal/library"
	"github.com/oneo-me/onevra/source/components"
	"github.com/oneo-me/onevra/source/components/markdown"
	"github.com/oneo-me/onevra/source/icons/lucide"
	"github.com/yuin/goldmark"
	"github.com/yuin/goldmark/ast"
	"github.com/yuin/goldmark/text"
)

type attachmentItem struct{ name, resource string }
type attachmentBlock struct {
	items   []attachmentItem
	invalid bool
}

// Parse only the legacy link-only syntax. Filesystem checks belong to activation,
// outside frame construction, and are repeated even for previously used links.
func parseAttachments(source string) attachmentBlock {
	data := []byte(source)
	doc := goldmark.New().Parser().Parse(text.NewReader(data))
	result := attachmentBlock{}
	invalid := func() attachmentBlock { return attachmentBlock{invalid: true} }
	for p := doc.FirstChild(); p != nil; p = p.NextSibling() {
		if _, ok := p.(*ast.TextBlock); ok && p.ChildCount() == 0 {
			continue
		}
		// Reference definitions resolve links and images but render nothing.
		if _, ok := p.(*ast.LinkReferenceDefinition); ok {
			continue
		}
		if _, ok := p.(*ast.Paragraph); !ok {
			return invalid()
		}
		for n := p.FirstChild(); n != nil; n = n.NextSibling() {
			if t, ok := n.(*ast.Text); ok && strings.TrimSpace(string(t.Segment.Value(data))) == "" {
				continue
			}
			link, ok := n.(*ast.Link)
			if !ok {
				return invalid()
			}
			target := string(link.Destination)
			if !strings.HasPrefix(target, "assets/") {
				return invalid()
			}
			resource, err := library.DecodeResource(target)
			if err != nil {
				return invalid()
			}
			parts := strings.Split(resource, "/")
			if len(parts) != 2 || parts[1] == "icon.png" {
				return invalid()
			}
			// Link labels may contain emphasis, but never embedded images or HTML.
			valid := true
			ast.Walk(link, func(child ast.Node, entering bool) (ast.WalkStatus, error) {
				if entering {
					switch child.(type) {
					case *ast.Link, *ast.Text, *ast.String, *ast.Emphasis, *ast.CodeSpan:
					default:
						valid = false
					}
				}
				return ast.WalkContinue, nil
			})
			if !valid || strings.TrimSpace(string(link.Text(data))) != parts[1] {
				return invalid()
			}
			result.items = append(result.items, attachmentItem{parts[1], resource})
		}
	}
	if len(result.items) == 0 {
		return invalid()
	}
	return result
}

func prepareAttachments(source string) map[string]attachmentBlock {
	data := []byte(source)
	doc := goldmark.New().Parser().Parse(text.NewReader(data))
	blocks := map[string]attachmentBlock{}
	ast.Walk(doc, func(n ast.Node, entering bool) (ast.WalkStatus, error) {
		if block, ok := n.(*ast.FencedCodeBlock); entering && ok && string(block.Language(data)) == "attachments" {
			code := string(block.Lines().Value(data))
			blocks[code] = parseAttachments(code)
		}
		return ast.WalkContinue, nil
	})
	return blocks
}

func (a *application) renderAttachments(c *ui.Context, block markdown.MarkdownCodeBlock) {
	parsed, ok := a.attachmentBlocks[block.Code]
	ui.Column(c).Key(fmt.Sprintf("attachments-%d", block.Index)).FillWidth().MinWidth(0).Gap(2).Children(func() {
		if !ok || parsed.invalid {
			components.Text(c, a.text("Could not show attachments")).TextColor(c.Theme().Danger)
			components.Text(c, a.text("Use only assets/filename links with matching filenames.")).Wrap()
			return
		}
		for i, item := range parsed.items {
			ui.Column(c).Key(fmt.Sprint(i)).FillWidth().MinWidth(0).Children(func() {
				components.Button(c, "", components.ButtonOutline).Label(item.name).FillWidth().MinWidth(0).Justify(ui.Start).Gap(8).OnClick(func() {
					a.revealAttachment(item)
				}).Children(func() {
					components.Icon(c, lucide.File()).Size(16, 16)
					components.Text(c, item.name).MinWidth(0).Wrap()
				})
			})
		}
	})
}

func (a *application) revealAttachment(item attachmentItem) {
	root, note := a.model.Root, a.model.Selected
	go func() {
		lib, err := library.Open(root)
		if err == nil {
			var notePath string
			notePath, err = lib.ResolvePath(note)
			if err == nil && !lib.Exists(note+"/"+item.resource) {
				err = fmt.Errorf("attachment is missing or unsafe: %s", item.name)
			}
			if err == nil {
				mygo.Shell.ShowItemInFolder(filepath.Join(notePath, filepath.FromSlash(item.resource)))
			}
			lib.Close()
		}
		if err != nil {
			a.post(func() {
				if a.model.Root == root && a.model.Selected == note {
					a.feedback = err.Error()
				}
			})
		}
	}()
}
