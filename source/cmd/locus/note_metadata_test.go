package main

import (
	"testing"

	"go.yaml.in/yaml/v3"
)

func TestNoteMetadataFields(t *testing.T) {
	var node yaml.Node
	err := yaml.Unmarshal([]byte("title: Test\nsummary: Description\nicon: book\ncontacts:\n  website: https://example.com\ntargets:\n  - name: macOS\n    enabled: true\n  - null\n  - [one, 2]\n"), &node)
	if err != nil {
		t.Fatal(err)
	}
	fields := noteMetadataFields(&node, true)
	if len(fields) != 2 || fields[0].label != "contacts" || fields[1].label != "targets" {
		t.Fatalf("custom fields: %+v", fields)
	}
	children := noteMetadataFields(fields[0].value, false)
	if len(children) != 1 || children[0].label != "website" || noteMetadataScalar(children[0].value) != "https://example.com" {
		t.Fatal("nested mapping lost")
	}
	targets := noteMetadataFields(fields[1].value, false)
	if len(targets) != 3 || targets[0].label != "[1]" || targets[2].label != "[3]" {
		t.Fatal("arrays must expand into one-based rows")
	}
	if noteMetadataScalar(targets[1].value) != "null" {
		t.Fatal("null lost")
	}
	nested := noteMetadataFields(targets[2].value, false)
	if len(nested) != 2 || noteMetadataScalar(nested[1].value) != "2" {
		t.Fatal("nested array lost")
	}
	if noteMetadataFields(nil, true) != nil {
		t.Fatal("nil metadata")
	}
}
func TestNoteMetadataScalars(t *testing.T) {
	for _, sample := range []struct{ yaml, want string }{{"{}", "{}"}, {"[]", "[]"}, {"null", "null"}, {"false", "false"}, {"42", "42"}, {"'line 1'", "line 1"}} {
		var node yaml.Node
		if err := yaml.Unmarshal([]byte(sample.yaml), &node); err != nil {
			t.Fatal(err)
		}
		if got := noteMetadataScalar(node.Content[0]); got != sample.want {
			t.Fatalf("%s => %q", sample.yaml, got)
		}
	}
}
