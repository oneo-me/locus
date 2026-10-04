# Settings and persistence

`locus-settings` owns the runtime preferences entity, settings pages, archive restoration, and settings events. `locus-preferences` defines fields, defaults, validation, and product option labels. The workspace consumes settings events without retaining another long-lived preference copy. Shared appearance controls own input state; product entities own configuration and save results.

## Settings pages

Settings replaces the workspace inside the main window. Returning restores its state and focus. The shared shell provides appearance settings and category navigation; Locus adds library, editor, export directory, archive restoration, fonts, terminal, Agent, Unix Shell, and MCP settings. Skills has its own navigation group and page with an installation location selector; selecting a location automatically installs the bundled skill and cleans up the old Locus link. The MCP page provides service enablement, listening scope, port, real status, and errors. See [Agent integration](mcp.md#skill-distribution-and-installation) for skill files and background installation and [MCP access](../mcp.md) for client connections.

An unconfigured export directory uses the system Downloads directory. Selection changes affect only subsequent exports or terminal launches; open pages and external sessions retain their own lifecycles. Save failures expose their reasons. Rollback for individual options follows their settings methods; values effective only in memory must not be described as saved.

## Configuration service

`SettingsStore::for_project(name)` uses `~/.oneo/<name>/settings.toml`. Names must be single non-empty directory names, excluding absolute paths, `.`, `..`, and separators. `SettingsStore::at_path(path)` replaces the complete file path, requires an absolute file path, and does not interpret shell syntax such as `~`. These are explicit alternative constructors. Locus uses the standard project name `locus`.

Missing configuration files return application defaults without creating a file. Application Serde types define missing-field behavior. Read and TOML format failures return errors with paths and do not overwrite files. On load failure, Locus uses default preferences and passes the reason to the existing settings error interface. Saving serializes first, creates parent directories, writes and synchronizes a temporary file in the same directory, then replaces the target. Failures do not truncate existing files. The service saves complete documents without preserving comments or fields outside the type; it provides no merging, watching, or migration. Callers serialize configuration changes. With concurrent writers, the last replacement wins. The settings entity remains the sole owner of runtime preferences.

`oneo-gpui` enables the `shell` feature by default, including app, window, and interface components. The configuration service does not depend on GPUI. Domain and preferences packages can use `default-features = false` to compile only configuration, appearance data types, and file I/O dependencies. `read` distinguishes missing files with `Option`; `load` returns defaults when missing; `create` creates atomically and refuses to overwrite; `save` replaces atomically. Theme Smith uses `create` for directory metadata to avoid overwriting other writers during concurrent creation.

## Appearance configuration

`AppearancePreferences` defines theme, language, and interface font size. Application preference types compose it with `#[serde(flatten)]`, producing top-level TOML fields `theme`, `language`, and `ui_font_size`. Theme defaults to `system`, with `light` and `dark` alternatives. Language defaults to `system`, with `chinese` and `english` alternatives. Interface font size defaults to 16 and supports 12, 14, 15, 16, 17, 18, and 20. Unsupported sizes normalize to the default after reading. Existing explicit language choices are not rewritten to system preference.

`sys-locale` reads system language at the process's first resolution. Chinese language identifiers use the Simplified Chinese catalog; other languages or lookup failure use English. Restart after changing the OS language. Explicit language choices or reselecting the system option immediately refresh the current interface and component locale. Product change events update menus and business text. System appearance changes apply immediately when following the system theme. Theme changes synchronize the native window, GPUI Theme, and Base projection while preserving user font size.

`AppearanceControls` owns selector state and subscriptions only. Application settings entities remain the sole owners of appearance configuration and save errors. Parents handle `AppearanceChange`, apply configuration, synchronize selectors, and save complete product preferences. The shared layer does not create another product preference copy or write to a fixed product path. Locus composes the three shared controls with its interface font, code font, and code size. Terminal, library, and MCP settings remain product responsibilities.

## Storage and write scheduling

- `~/.oneo/locus/settings.toml`: library, editor, export directory, appearance, language, fonts, terminal, Agent shell, and MCP preferences.
- `~/.oneo/locus/state.toml`: sidebar state, current selection, and normal-window restoration bounds.
- `~/.oneo/locus/sidebar-order.toml`: sidebar sibling order by library and parent path, without modifying library content.
- `~/.oneo/locus/thumbnails/` and `link-icons/`: local caches rebuildable from their sources.

Project-owned Rust configuration uses TOML, not JSON. Settings, interface state, and sidebar order all use the shared configuration service. Old terminal fields and `auto_open_canvas` are no longer valid preferences. Normal complete serialization removes them while retaining other valid preferences; existing user configuration is not proactively overwritten. Settings, interface state, and caches cannot become authorities for note content, hierarchy, or relationships.

Window moves and resizes update only in-memory workspace bounds. Each event resets a 100 ms delay, which submits the latest interface state after activity stops. `InterfaceStateWriter` passes complete snapshots of window bounds, sidebar state, and selection to GPUI's background executor. It serializes TOML, writes atomically, and synchronizes disk in submission order, avoiding UI-thread disk writes and old snapshots overwriting new state. Shutdown cancels a pending delay, submits the latest snapshot, and waits for the queue. GPUI's 200 ms shutdown limit bounds that wait; unusually slow disks may prevent the final save from finishing. Save failures use existing standard-error reporting.

## Language resources

Packages owning interface copy maintain their own `locales/ui.yml`, invoke `rust_i18n::i18n!("locales", fallback = "en")` at their crate roots, and resolve strings locally through `t!`. `locus-workspace` owns workspace, menu, and terminal-launch product copy; `locus-settings` owns product settings copy; `oneo-gpui` owns shared settings and appearance controls; `locus-ui` owns note extension copy; `locus-canvas` owns canvas loading and error copy; `locus-preferences` owns preference option labels. Each package keeps only its own resources in English and Simplified Chinese. Packages do not proxy other packages' keys through preferences or rely on cross-package catalog merging. Shared components consuming caller labels without their own copy do not create empty language packages.

`oneo_gpui::Language` provides language preferences, locale resolution, option names, and unified switching. Callers explicitly select language with `t!(..., locale = language.locale())` and use named macro arguments for substitutions. Switching synchronizes component global locale, rendered views, and native menus; global locale changes alone do not trigger GPUI redraws. `extend!` registers once under component namespaces only when overriding component copy is necessary; no empty extensions are currently registered. Usage follows the [GPUI Kit internationalization guidelines](https://gpui-kit.com/zh-CN/docs/i18n/).

Each resource package declares `cargo:rerun-if-changed=locales` in its `build.rs`, ensuring YAML-only additions, edits, and removals regenerate compile-time translations. Package tests verify resolved values for every resource in both languages. Note UI tests cover literal placeholders within filenames.

## Verification boundaries

Check field defaults, TOML round trips, invalid-size normalization, missing and damaged files, atomic creation and replacement, write ordering, and failure feedback. Workspace tests must verify that blocking an old write still preserves ordered saves of the latest snapshot.

Actual window-drag responsiveness, restoration after quitting, system appearance changes, bilingual menus, and focus across settings require target-platform confirmation. Shutdown saving on unusually slow disks remains subject to the limit above.
