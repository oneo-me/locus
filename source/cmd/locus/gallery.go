package main

import (
	"net/url"
	"strings"

	"github.com/egoist/mygo/ui"
	"github.com/egoist/mygo/ui/markdown"
	"github.com/oneo-me/onevra/source/components"
	"github.com/yuin/goldmark"
	"github.com/yuin/goldmark/ast"
	"github.com/yuin/goldmark/extension"
	"github.com/yuin/goldmark/text"
	"github.com/yuin/goldmark/util"
)

type galleryItem struct{ Target, Alt string }
type galleryBlock struct {
	Items []galleryItem
	Error string
}

func decodeMarkdown(value []byte) string {
	return string(util.ResolveEntityNames(util.ResolveNumericReferences(util.UnescapePunctuations(value))))
}

// Gallery resources stay inside the selected note's assets directory. Bare
// filenames retain the old syntax while using the current attachment layout.
func galleryTarget(target string) string {
	u, err := url.Parse(target)
	if err != nil || u.Scheme != "" || u.Host != "" || u.RawQuery != "" || u.ForceQuery || strings.Contains(target, "#") {
		return ""
	}
	p, err := url.PathUnescape(u.EscapedPath())
	if err != nil || strings.ContainsAny(p, "\\\x00") {
		return ""
	}
	if !strings.Contains(p, "/") {
		p = "assets/" + p
	}
	if !strings.HasPrefix(p, "assets/") {
		return ""
	}
	for _, part := range strings.Split(p, "/") {
		if part == "" || part == "." || part == ".." {
			return ""
		}
	}
	return (&url.URL{Path: p}).String()
}

func parseGallery(source string) galleryBlock {
	data := []byte(source)
	root := goldmark.New().Parser().Parse(text.NewReader(data))
	result := galleryBlock{}
	invalid := func() galleryBlock {
		return galleryBlock{Error: "Include only local images with alt text in the gallery."}
	}
	for block := root.FirstChild(); block != nil; block = block.NextSibling() {
		if _, ok := block.(*ast.TextBlock); ok && block.FirstChild() == nil {
			continue
		}
		// Reference definitions resolve links and images but render nothing.
		if _, ok := block.(*ast.LinkReferenceDefinition); ok {
			continue
		}
		if _, ok := block.(*ast.Paragraph); !ok {
			return invalid()
		}
		for node := block.FirstChild(); node != nil; node = node.NextSibling() {
			if whitespace, ok := node.(*ast.Text); ok && strings.TrimSpace(string(whitespace.Segment.Value(data))) == "" {
				continue
			}
			im, ok := node.(*ast.Image)
			if !ok {
				return invalid()
			}
			target, alt := galleryTarget(decodeMarkdown(im.Destination)), decodeMarkdown(im.Text(data))
			if target == "" || strings.TrimSpace(alt) == "" {
				return invalid()
			}
			result.Items = append(result.Items, galleryItem{target, alt})
		}
	}
	if len(result.Items) == 0 {
		result.Error = "Include at least one image in the gallery."
	}
	return result
}

// Fenced galleries are keyed by their raw fence content so that rendering can
// match MyGo's parsed blocks without depending on block indices.
func markdownResources(body string) (map[string]galleryBlock, []string) {
	data := []byte(body)
	root := goldmark.New(goldmark.WithExtensions(extension.GFM)).Parser().Parse(text.NewReader(data))
	galleries := map[string]galleryBlock{}
	var targets []string
	_ = ast.Walk(root, func(n ast.Node, entering bool) (ast.WalkStatus, error) {
		if !entering {
			return ast.WalkContinue, nil
		}
		switch n := n.(type) {
		case *ast.Image:
			targets = append(targets, decodeMarkdown(n.Destination))
		case *ast.FencedCodeBlock:
			var language string
			if n.Info != nil {
				if fields := strings.Fields(decodeMarkdown(n.Info.Text(data))); len(fields) > 0 {
					language = fields[0]
				}
			}
			if language == "gallery" {
				code := string(n.Lines().Value(data))
				gallery := parseGallery(code)
				galleries[code] = gallery
				for _, item := range gallery.Items {
					targets = append(targets, item.Target)
				}
			}
		}
		return ast.WalkContinue, nil
	})
	return galleries, targets
}

func (a *application) renderGallery(c *ui.Context, block *markdown.CodeBlockView) {
	gallery := a.galleries[block.Code()]
	if gallery.Error != "" {
		components.Text(c, a.text(gallery.Error)).Wrap().TextColor(c.Theme().Danger)
		return
	}
	ui.Row(c).FillWidth().MinWidth(0).Wrap().Gap(8).Children(func() {
		for i, item := range gallery.Items {
			bitmap := a.images[item.Target]
			width := float32(180)
			if bitmap != nil {
				w, h := bitmap.Size()
				width = min(360, max(120, 180*float32(w)/float32(h)))
			}
			button := components.Button(c, "", components.ButtonGhost).Key(i).Label(item.Alt).
				Basis(width).MaxWidthPercent(100).MinWidth(0).Height(180).Padding(0).Radius(8).Clip().Disabled(bitmap == nil)
			button.Children(func() {
				if bitmap != nil {
					components.Image(c, bitmap).Fill().Fit(ui.Cover).Label(item.Alt)
				} else {
					components.Text(c, item.Alt+"\n"+a.text("Image unavailable")).Wrap().TextColor(c.Theme().TextMuted)
				}
			})
			if button.Clicked() {
				a.galleryPreview = item
				a.galleryPreviewOpen = true
			}
		}
	})
}

func (a *application) galleryDialog(c *ui.Context) {
	if !a.galleryPreviewOpen {
		return
	}
	bitmap := a.images[a.galleryPreview.Target]
	if bitmap == nil {
		a.galleryPreviewOpen = false
		return
	}
	w, h := c.Size()
	components.Dialog(c, &a.galleryPreviewOpen, components.DialogOptions{Label: a.galleryPreview.Alt}, func(panel ui.Element) {
		panel.Width(max(0, w-80)).Height(max(0, h-100)).Children(func() {
			components.Image(c, bitmap).FillWidth().Grow(1).MinHeight(0).Fit(ui.Contain).Label(a.galleryPreview.Alt)
			components.Text(c, a.galleryPreview.Alt).Wrap()
			components.Button(c, a.text("Close"), components.ButtonGhost).OnClick(func() { a.galleryPreviewOpen = false })
		})
	})
}
