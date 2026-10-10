package main

import (
	"fmt"
	"math"
	"strings"

	"github.com/egoist/mygo/ui"
	"github.com/oneo-me/onevra/components"
	"go.yaml.in/yaml/v3"
)

type noteMetadataField struct {
	label string
	value *yaml.Node
}

func noteMetadataFields(node *yaml.Node, top bool) []noteMetadataField {
	if node == nil {
		return nil
	}
	if node.Kind == yaml.DocumentNode && len(node.Content) > 0 {
		node = node.Content[0]
	}
	var fields []noteMetadataField
	switch node.Kind {
	case yaml.MappingNode:
		for i := 0; i+1 < len(node.Content); i += 2 {
			key := node.Content[i].Value
			if top {
				switch key {
				case "title", "summary", "icon":
					continue
				}
			}
			fields = append(fields, noteMetadataField{key, node.Content[i+1]})
		}
	case yaml.SequenceNode:
		if !top {
			for i, v := range node.Content {
				fields = append(fields, noteMetadataField{fmt.Sprintf("[%d]", i+1), v})
			}
		}
	}
	return fields
}
func noteMetadataScalar(node *yaml.Node) string {
	if node == nil {
		return "null"
	}
	switch node.Kind {
	case yaml.MappingNode:
		return "{}"
	case yaml.SequenceNode:
		return "[]"
	case yaml.AliasNode:
		return "*" + node.Value
	}
	if node.Tag == "!!null" {
		return "null"
	}
	return node.Value
}

// Summary and recursive YAML rows share one surface, as in note_metadata.rs.
func noteMetadataPanel(c *ui.Context, summary string, node *yaml.Node, color ui.Color) {
	fields := noteMetadataFields(node, true)
	if strings.TrimSpace(summary) == "" && len(fields) == 0 {
		return
	}
	ui.Column(c).FillWidth().MinWidth(0).Radius(8).Clip().Background(color).Children(func() {
		if strings.TrimSpace(summary) != "" {
			components.Text(c, summary).FillWidth().MinWidth(0).Padding(8, 14).FontSize(14).FixedLineHeight(21).Wrap().TextColor(c.Theme().Text)
		}
		if len(fields) > 0 {
			ui.Column(c).FillWidth().MinWidth(0).Padding(4, 0).Children(func() { noteMetadataList(c, fields, 0) })
		}
	})
}
func noteMetadataList(c *ui.Context, fields []noteMetadataField, depth int) {
	width := float32(0)
	for _, field := range fields {
		for _, line := range strings.Split(field.label, "\n") {
			w, _ := c.MeasureText(0, ui.Span{Text: line, Size: 12, Weight: 500})
			width = max(width, w)
		}
	}
	width = float32(math.Ceil(float64(width + 28)))
	for _, field := range fields {
		ui.Row(c).FillWidth().MinWidth(0).AlignItems(ui.Start).Children(func() {
			components.Text(c, field.label).Width(width).Shrink(0).Padding(2, 14).FontSize(12).FontWeight(500).FixedLineHeight(18).TextColor(c.Theme().TextMuted)
			ui.Column(c).Grow(1).MinWidth(0).Children(func() {
				children := noteMetadataFields(field.value, false)
				if len(children) > 0 && depth < 32 {
					noteMetadataList(c, children, depth+1)
				} else {
					components.Text(c, noteMetadataScalar(field.value)).FillWidth().MinWidth(0).Padding(2, 14).FontSize(12).FixedLineHeight(18).Wrap().TextColor(c.Theme().Text)
				}
			})
		})
	}
}
