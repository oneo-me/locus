# MCP access

Locus runs two Streamable HTTP endpoints on one listener. Both use the configured library, port, service switch and local-network setting.

| Endpoint | Default local URL | Contract |
| --- | --- | --- |
| Standard | `http://127.0.0.1:2301/mcp` | `list_articles`, `search_articles`, `resolve_article`; metadata and absolute local paths for clients with filesystem access. |
| Full access | `http://127.0.0.1:2301/mcp/full` | Library operations and content transfer using logical addresses; no server filesystem paths in generated responses or errors. |

The service is disabled by default. Local-network mode exposes both endpoints without authentication. Connected network clients can read, upload, modify and permanently delete library content; the standard endpoint also returns local paths. Both endpoints use the same Host restrictions. Changing the library or stopping/restarting the service invalidates existing sessions and pending uploads. Settings provides the service switch, local-network switch, port and runtime status. The sidebar also shows status. Connection guides, URL display and copying are not provided in the application. Configure clients with the URLs above, replacing the port when changed and the host with this computer’s IP for network access. Host validation accepts localhost, loopback addresses and the local IP receiving each connection; arbitrary hostnames are rejected.

The full endpoint never asks the client to edit server files. A client still needs to supply upload bytes and save download bytes. Original note text and user metadata are returned faithfully, including any paths already written by the author.

## Built-in software documentation

Both endpoints expose read-only English Markdown through MCP `resources/list` and `resources/read`:

| URI | Specification |
| --- | --- |
| `locus-docs://attachments` | Attachment ownership, gallery and file-reference syntax. |
| `locus-docs://canvas` | Canvas authoring, lifecycle, local/network resources and templates. |

Read the relevant resource before authoring note attachments or canvases. These addresses identify software documentation, not library notes; do not pass them to article or content tools. Resources remain available when no library is selected and cannot be overridden by library content. Unknown URIs return a resource-not-found error.

The source files live in [`resources/docs`](../../resources/docs). The same files are embedded at compile time for MCP access and copied by the packer to `Contents/Resources/docs` on macOS or `resources/docs` in the Windows portable directory. Documentation therefore follows the running application version and requires no separate file access by the client.

## Standard discovery paths

`resolve_article` accepts a library-relative path or a `locus://` address in its `path` parameter, for example `{"path":"locus://projects/example"}`. Either form may end in `/index.md`. `list_articles.parent` accepts the same forms; an empty parent or `locus://` lists the library root. The root cannot be resolved as an article. Returned `path` values remain library-relative.

Logical addresses use the same single percent-decoding pass as application navigation, including encoded spaces, `%` and `#`. Invalid address segments and encoded traversal are rejected. Plain relative paths are not percent-decoded.

## Addresses and content

A note has a logical address such as `locus://projects/example`. `locus://` is the library root for browsing and export. Names follow portable filename rules. Spaces, `%` and `#` use percent encoding in addresses; the application decodes each address once for navigation and canvas templates. A content target identifies its owner and resource:

```json
{"uri":"locus://projects/example","resource":{"kind":"note"}}
{"uri":"locus://projects/example","resource":{"kind":"canvas"}}
{"uri":"locus://projects/example","resource":{"kind":"icon"}}
{"uri":"locus://projects/example","resource":{"kind":"attachment","name":"diagram.png"}}
```

The five resource kinds are `note` (`index.md`), `canvas` (`canvas/index.html`), `canvas_file` (any other relative file path under `canvas/`), `icon` (`assets/icon.png`) and `attachment` (a filename under `assets/`). Canvas resources may include nested directories, scripts, styles, images and data. `assets` and `canvas` are reserved directory names and cannot identify notes. Attachment filenames cannot address the reserved icon. Templates are other notes' canvas resources, read through the same tools; HTML canvases require `canvas/main.js` exporting `async main(data)`. Template callers instead write `canvas/index.json` with exactly `template` (a `locus://` note address) and `data`. Use the `canvas_file` target for this JSON entry and main module. HTML and JSON entries are mutually exclusive; relative resources belong to the template.

Hidden and excluded directories, symlinked entries, traversal and nonportable names are rejected. A directory with unmanaged content cannot be moved or recursively deleted through this endpoint. Neither root deletion nor root renaming is available.

## Tools

| Tools | Purpose |
| --- | --- |
| `list_entries`, `search_notes`, `resolve_entry` | Discover notes, organizational directories, archives, metadata and resources. Listing and search support cursor pagination. |
| `create_directory`, `create_note` | Create in an existing parent. A note can be added to an organizational directory without replacing its children. |
| `read_text`, `write_text`, `patch_text` | Read and edit UTF-8 text, including full YAML frontmatter and custom metadata. Patching requires one exact match. |
| `stat_content`, `download_chunk` | Obtain size/version and download byte-exact content. |
| `begin_upload`, `upload_chunk`, `finish_upload`, `cancel_upload` | Stage, validate, commit or discard binary/text uploads. |
| `set_archived` | Archive or restore a note without rewriting unrelated metadata or body. |
| `delete_content` | Permanently remove one resource. Deleting only the note body preserves its directory and other files; recreate the note body to access its owned resources again. |
| `plan_move`, `move_entry` | Preview then move/rename a note or organizational subtree, updating resolvable Markdown links. |
| `move_attachment` | Rename or move an attachment to another note without overwriting, updating recognized Markdown links and returning affected notes. Review canvas/script references with `patch_text`. |
| `inspect_tree`, `delete_entry` | Inspect scope and revision, then remove an empty directory or explicitly delete the entire subtree. |
| `export_manifest` | Enumerate directories and versioned resources for subtree export and reconstruction. |

Tool schemas and initialization instructions describe required parameters. Tool errors use `isError`, with readable codes such as `conflict`, `invalid_uri`, `unsafe_target`, `incomplete_upload` and `checksum_mismatch`.

## Editing and conflicts

Read operations return a SHA-256 content `version`. Supply it as `expected_version` for changes and deletions. For creation, use `expected_version: "missing"`; existing content is never implicitly overwritten. A stale version returns `conflict`, and the client must read and reconcile the current content before retrying.

`read_text` uses zero-based `start_line`, defaults to 200 lines and accepts up to 2,000 lines per result. Its version always covers the complete file. `nextLine` indicates additional content; concatenate all chunks before a full replacement, or use an exact `patch_text`. Direct text operations and uploaded notes/canvases are limited to 8 MiB. Existing larger files can be downloaded as bytes.

All full-endpoint operations serialize through the service's library owner. Writes stage a complete file, verify the expected version again, then replace the target. This is optimistic conflict detection for external editors, not a filesystem transaction or lock protocol those editors participate in. A process crash during a multi-file operation is not covered by an atomic transaction.

`plan_move` returns a version that includes the source tree and affected note revisions. `move_entry` requires that version. It updates recognized inline/reference Markdown destinations, including inbound links and relative outbound links from moved notes. Code spans/blocks and frontmatter are preserved. Canvas HTML, scripts, custom metadata and dynamically constructed references require explicit review. The move result identifies the affected notes. If a link write fails, the service attempts rollback; an incomplete rollback returns `partial_failure` and requires inspection of both logical addresses.

`delete_entry` requires the version returned by `inspect_tree`. Its default `recursive: false` only removes an empty directory. `recursive: true` permanently removes the complete inspected subtree. `delete_content` removes exactly the specified resource. Deleting an icon clears its metadata reference. Deletion does not erase inbound links from other notes.

## Upload and download

1. Call `begin_upload` with a target, decoded byte `size`, optional hexadecimal `sha256`, and `expected_version`.
2. Send `upload_chunk` with the returned `upload_id`, current byte `offset` and base64 content. Each chunk is at most 256 KiB decoded. An exact retry of already received bytes is accepted; mismatching or overlapping retries fail.
3. Call `finish_upload`. The size and optional checksum must match; the destination version is checked before committing. An incomplete or invalid upload leaves the destination untouched. `cancel_upload` discards staged bytes.

Each session permits four pending uploads with a combined declared size of at most 1 GiB. Uploads expire after ten idle minutes; expired transfers are removed on the next operation. Service/session teardown also releases their temporary files. These temporary files are outside the content library until commit.

Icon uploads use the application's image normalization (at most 16 MiB source, proportional PNG output up to 64 × 64), and update the note metadata. The returned resource version describes the normalized icon; `uploadedSha256` describes the submitted bytes.

To download, call `stat_content`, then repeatedly call `download_chunk` with its `version`, an offset starting at zero, and an optional byte limit. The default is 64 KiB and the maximum is 256 KiB. Decode and append the base64 data, advance to `nextOffset`, and stop at `eof`. A changed file fails instead of mixing revisions.

For a subtree, obtain `export_manifest`, download each listed resource at its listed version and retain the manifest with those bytes. To import, recreate directories parent-first, create each note from its downloaded Markdown, then upload its canvas and attachments. Apply icon uploads using the normalization contract above. Re-export and compare versions to detect changes during an export or import. This workflow uses the same MCP endpoint throughout; it does not require a server-side path or separate HTTP download URL.

## Verification

```sh
cargo test -p locus-library -p locus-mcp
```

The HTTP tests use temporary libraries and local ephemeral ports, exercise standard/full tool separation and a complete content workflow, and compare uploaded/downloaded bytes. Pure tests cover version conflicts, traversal, symlinks, text preservation, archive restore, canvas consumption, subtree moves and upload failures. Settings checks and user interface review are separate from protocol tests.
