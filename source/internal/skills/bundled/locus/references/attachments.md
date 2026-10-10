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

## Files

Use ordinary Markdown links to actual resources:

```markdown
[field-summary.pdf](assets/field-summary.pdf)
[source-data.csv](assets/source-data.csv)
```

Relative file links reveal the validated resource in the system file manager rather than executing an attachment. Do not use absolute filesystem paths or links escaping the selected library. Keep attachment descriptions in surrounding text.

Dedicated `gallery` and `attachments` rendering extensions are not implemented; those fences appear as code rather than interactive resource lists. Use standard Markdown instead. Icon upload and icon rendering are not part of the current reader, but preserve existing icon files when maintaining a note.

For canvas-owned resources and fetch, read [canvas.md](canvas.md).
