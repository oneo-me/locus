# Reading notes

The selected note's body renders through MyGo's Markdown view inside the [desktop workspace](workspace.md). This subject owns the reading surface: Markdown projections, article backgrounds, local images, galleries, and attachment rows.

## Markdown view

MyGo's Markdown view parses the body incrementally and keeps unchanged blocks across frames. The app's own projections of gallery and attachment fences, together with decoded images, are recomputed only when the selected note or body changes. Summaries and custom YAML share one tinted, clipped panel above the body. YAML mappings and arrays expand into recursive label/value rows rather than serialized YAML or an editable property form. Local Markdown images are decoded outside frame builders through the library resource boundary, with count, file-size, and pixel limits. Remote images are not fetched automatically. MyGo's Markdown view owns selectable text, tables, and code presentation.

## Article backgrounds

Article backgrounds derive a stable cool palette, one of seven backdrop variants, and four to six geometric accents from the note title. Layout tables, shape families, deterministic jitter, and per-shape tones determine the composition. Light and dark themes change colors without moving geometry. The wash fades across 268 logical pixels, while texture stays at full strength through the first 70% before fading. Shapes use a centered 360 × 100 design space capped at 208 pixels high, with separate vertical and horizontal fades.

Decorations scroll with the article, do not intercept text interaction, and require no animation loop. Canvas pages retain their own presentation.

## Galleries

The exact, case-sensitive `gallery` fence renders Markdown images as wrapping, 180-pixel-high thumbnails. Pointer or button keyboard activation opens a full-window, contain-fit preview with no panel surface, alternative text, and a close control in the window's top-right corner; the control, Escape, or backdrop dismissal closes it. Image syntax, reference definitions, source order, and repeated images are supported. Empty blocks, missing alternative text, non-image content, and non-local destinations display a localized validation error. Missing or unsupported images retain an unavailable placeholder.

Gallery parsing is cached with the selected Markdown body and uses MyGo's custom code-block builders. The shared asynchronous image loader discovers gallery destinations as well as ordinary images, deduplicates decoded targets, limits a note to 64 pictures and 48 million retained pixels, and scales larger pictures down to share that budget instead of marking them unavailable; sources beyond 64 million pixels are not decoded. Builders do no file I/O. Note changes and image reloads dismiss previews; generation tickets reject stale image results. Remote gallery resources are rejected.

Gallery destinations belong to the current note's `assets/` directory; legacy bare filenames resolve there. Existing files stored beside `index.md` must be moved into `assets/` explicitly before they can load. Absolute paths, traversal, cross-note addresses, queries, and fragments are rejected. The [bundled attachment reference](../../source/resources/skills/locus/references/attachments.md) owns authoring examples.

## Attachments

Legacy `attachments` fences render ordered, keyboard-accessible file buttons through MyGo's custom code-block interface. The cached parser accepts only Markdown links to direct `assets/<filename>` children with matching filename labels, excluding the reserved icon. Invalid blocks show a localized explanation. Activation checks the owning note and regular file through rooted library access in a background task, then reveals it in the system file manager; missing or unsafe files report an error. Rendering performs no filesystem I/O.

## Acceptance boundaries

Atmosphere parity fixtures independently evaluate the reference formulas and tables for all six layouts, ten shape families, seven backdrop variants, and both themes, including CJK and astral titles. Native captures exercise the reference titles at two window widths in both themes.

These checks establish numeric source parity and exercise MyGo rendering, not pixel equivalence with GPUI. Arc paths use cubic Béziers, and colors enter MyGo's color representation at paint time. Texture cells are submitted as small paths rather than viewport-wide rotated masks, avoiding eviction of text glyphs from the shared rendering atlas. Captures of the MyGo surface alone do not include OS-composited materials or prove browser or toolkit overlay geometry.

Gallery tests cover fenced-block dispatch, syntax validation, local-path restrictions, missing-image placeholders, and preview opening and dismissal. The macOS reading smoke test loads gallery images, captures thumbnails and a preview, and verifies dismissal on note departure. A full native-suite run encountered a MyGo accessibility list-state nil dereference; the isolated reading test passed. This does not establish complete native keyboard or accessibility acceptance.

The native reading fixture includes legacy attachment rows in light and dark captures. File-manager activation and full keyboard acceptance remain manual checks. Native icon checks also process a large PNG, update a temporary note, and verify watcher-driven icon refresh.
