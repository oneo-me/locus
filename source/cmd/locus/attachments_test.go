package main

import "testing"

func TestAttachmentSyntax(t *testing.T) {
	for _, source := range []string{
		"[report.pdf](assets/report.pdf)",
		"[中文 文件.pdf](assets/中文%20文件.pdf)\n\n[report.pdf](assets/report.pdf)",
		"[**report**.pdf](assets/report.pdf)\n[report.pdf](assets/report.pdf)",
		"[report.pdf][file]\n\n[file]: assets/report.pdf",
	} {
		got := parseAttachments(source)
		if got.invalid || len(got.items) == 0 {
			t.Errorf("valid block rejected: %q", source)
		}
	}
	for _, source := range []string{
		"", "plain text", "[Report](assets/report.pdf)", "![report.pdf](assets/report.pdf)",
		"- [report.pdf](assets/report.pdf)", "# Heading", "[report.pdf](https://example.com/report.pdf)",
		"[report.pdf](../assets/report.pdf)", "[report.pdf](assets/../report.pdf)",
		"[report.pdf](assets/%2e%2e/report.pdf)", "[report.pdf](assets/sub%2freport.pdf)",
		"[report.pdf](assets/sub/report.pdf)", "[report.pdf](assets/report.pdf?download=1)",
		"[report.pdf](assets/report.pdf#section)", "[icon.png](assets/icon.png)",
		"[index.md](index.md)", "[report.pdf](assets/report.pdf) extra",
		"[report.pdf](assets/report.pdf)\n\n![image](assets/image.png)",
	} {
		got := parseAttachments(source)
		if !got.invalid || len(got.items) != 0 {
			t.Errorf("invalid block accepted: %q", source)
		}
	}
	got := parseAttachments("[中文 文件.pdf](assets/中文%20文件.pdf)\n[中文 文件.pdf](assets/中文%20文件.pdf)")
	if len(got.items) != 2 || got.items[0].resource != "assets/中文 文件.pdf" || got.items[0] != got.items[1] {
		t.Fatalf("order, duplicates, or URL decoding lost: %#v", got)
	}
}

func TestAttachmentFenceDispatch(t *testing.T) {
	source := "~~~attachments\n[report.pdf](assets/report.pdf)\n~~~\n\n" +
		"> ```attachments\n> [data.csv](assets/data.csv)\n> ```\n\n" +
		"```go\n[ignored](assets/ignored)\n```\n\n" +
		"```Attachments\n[ignored](assets/ignored)\n```\n"
	blocks := prepareAttachments(source)
	if len(blocks) != 2 {
		t.Fatalf("unexpected blocks: %#v", blocks)
	}
	for _, block := range blocks {
		if block.invalid {
			t.Fatal("valid nested fence rejected")
		}
	}
}
