# Attachments

A note owns the files in its `assets/` directory. Keep `assets/icon.png` reserved for note-icon data. Files under `canvas/` are canvas resources, not child notes. Do not move unrelated library files into a note just to make a reference work.

Resolve the note through MCP before editing. Use the client's filesystem tools to add files and preserve their original names unless the user requests a rename. Never overwrite an existing attachment without inspecting it and confirming the intended replacement.

## Images

Use ordinary Markdown image paragraphs, with meaningful alternative text:

```markdown
![Materials and tools on a workbench](assets/workbench.jpg)

![Detail of the woven fabric](assets/woven-detail.jpg)
```

Locus loads local images through the note-resource boundary and shrinks them to the reading width. Inline images may display alternative text. Remote images are not fetched automatically by the Markdown reader. Large or unsupported images retain their alternative text. Keep headings and captions in surrounding Markdown.

## Galleries

Use a `gallery` fence to group local images into wrapping thumbnails. Each image needs meaningful alternative text. Activate a thumbnail to open a larger preview; use Close, Escape, or the backdrop to dismiss it.

````markdown
```gallery
![Materials and tools on a workbench](assets/workbench.jpg)
![Detail of the woven fabric](assets/woven-detail.jpg)
```
````

Separate images with newlines or blank lines. Markdown image reference definitions also work. Use only images inside this note's `assets/` directory. Legacy bare filenames such as `workbench.jpg` resolve to `assets/workbench.jpg`; files still beside `index.md` need an explicit move into `assets/`. Remote URLs, cross-note addresses, absolute paths, and traversal are rejected. Invalid gallery content shows an error; unavailable or oversized images show placeholders. The gallery shares the reader's local-image loading limits.

## Files

Use ordinary Markdown links to actual resources:

```markdown
[field-summary.pdf](assets/field-summary.pdf)
[source-data.csv](assets/source-data.csv)
```

Relative file links reveal the validated resource in the system file manager rather than executing an attachment. Do not use absolute filesystem paths or links escaping the selected library. Keep attachment descriptions in surrounding text.

Dedicated `attachments` fences still appear as code rather than interactive resource lists. Use standard Markdown file links instead. The sidebar and page title render an Emoji from the frontmatter `icon` string, or the reserved PNG when `icon: assets/icon.png` is set (`icon.png` is accepted as shorthand). Missing or invalid images fall back to the default note icon. The note’s More actions menu offers Upload icon for PNG files, scaling large images proportionally to fit 64 × 64 and updating the reserved file and frontmatter reference. Preserve existing icon files when maintaining unrelated content.

For canvas-owned resources and fetch, read [canvas.md](canvas.md).
