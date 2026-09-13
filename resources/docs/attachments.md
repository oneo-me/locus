# Attachments

A note is a directory containing `index.md`. Ordinary files in that same directory are its attachments. `index.md`, `icon.png`, and `canvas.html` are reserved note resources, not ordinary attachments.

Reference existing attachments by their actual same-directory filenames. Do not use absolute paths, parent paths, or missing files. Items are displayed in reference order.

## Images

Use a `gallery` fenced block for one image or a gallery. Include only Markdown images, each with meaningful alternative text:

````markdown
```gallery
![Materials and tools on a workbench](workbench.jpg)
![Detail of the woven fabric](woven-detail.jpg)
```
````

## Files

Use an `attachments` fenced block for files of any format. Include only Markdown links. The link label must match the filename exactly:

````markdown
```attachments
[field-summary.pdf](field-summary.pdf)
[source-data.csv](source-data.csv)
[source-bundle.zip](source-bundle.zip)
```
````

Keep headings, counts, captions, and explanations outside these blocks, in the surrounding Markdown.

## Agent access

On the standard MCP endpoint, resolve the note and use the client's filesystem tools to maintain its files. On the full MCP endpoint, use the note's `attachment` content target with the exact filename and the text or upload tools. Use the separate `icon` and `canvas` resource kinds for those reserved resources. Respect versions when modifying content through the full endpoint.

Canvas resource references use HTML and browser URL encoding; read the built-in `locus-docs://canvas` MCP resource for that contract.
