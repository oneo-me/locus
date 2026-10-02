# Attachments

A note is a directory containing `index.md`. Ordinary files in its `assets/` directory are its attachments. `assets/icon.png` is the reserved note icon and is excluded from attachments. `canvas/index.html` is the canvas entry; all files under `canvas/` belong to the canvas. `assets` and `canvas` are reserved directory names and cannot identify notes.

Reference existing attachments by their actual `assets/<filename>` paths. Do not use absolute paths, parent paths, or missing files. Items are displayed in reference order.

## Images

Use a `gallery` fenced block for one image or a gallery. Include only Markdown images, each with meaningful alternative text:

````markdown
```gallery
![Materials and tools on a workbench](assets/workbench.jpg)
![Detail of the woven fabric](assets/woven-detail.jpg)
```
````

## Files

Use an `attachments` fenced block for files of any format. Include only Markdown links. The link label must match the filename exactly:

````markdown
```attachments
[field-summary.pdf](assets/field-summary.pdf)
[source-data.csv](assets/source-data.csv)
[source-bundle.zip](assets/source-bundle.zip)
```
````

Keep headings, counts, captions, and explanations outside these blocks, in the surrounding Markdown.

## Agent access

Resolve the note with MCP and use the client's filesystem tools to maintain its files. Keep the reserved icon separate from ordinary attachments.

For canvas references, read [canvas.md](canvas.md).
