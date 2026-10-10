# Running and building

Locus has one Go module under `source/` and uses Onevra's Go packager. No Rust toolchain is required or used. Read the [architecture overview](../ARCHITECTURE.md) and [AGENTS](../AGENTS.md) before changing mechanisms.

## Dependencies

Use Go 1.27.1 or later and a native desktop session. `source/go.mod` pins published Onevra `v0.2026.2` and MyGo `v0.4.0`; `source/go.sum` records their checksums. WebView uses MyGo's built-in API, so no local replacements, patched checkouts, or preparation scripts are needed.

Download dependencies from `source/`:

```sh
GOWORK=off go mod download
```

macOS requires macOS 13 or later; native packaging needs Xcode and Command Line Tools. MyGo also provides Windows and Linux WebView backends. Those platforms require MyGo's native dependencies and separate Locus verification.

## Running

From the repository root:

```sh
cd source
GOWORK=off go run ./cmd/locus
GOWORK=off go run ./cmd/locus -library /path/to/library
```

The library can also be chosen through Open library. Existing preferences load from `~/.oneo/locus/settings.yaml`. Settings and note operations affect real local data; development uses the same product storage unless a test supplies isolated fixtures. Tests do not operate on the user's library.

The app watches external file changes. Canvas scripts rerun only after explicit refresh or reopening. Native canvases use a custom protocol; the app does not start an HTML development server or install Node dependencies.

## Building the app

From `source/`:

```sh
GOWORK=off go tool packer run
GOWORK=off go tool packer build
```

VS Code `run` and `build` tasks invoke these commands. [source/packer.yaml](../source/packer.yaml) targets Apple Silicon macOS by default and outputs to `dist/`. The application identity is Locus. After a successful build, the packager automatically opens `dist/releases/<version>/` containing the final archives in the system file manager. Set `openReleases: false` in `source/packer.yaml` to disable this for unattended builds. Explicitly configure and verify additional target architectures before claiming support.

See [App packaging](architecture/packaging.md) for resource ownership and platform boundaries. Skills are ordinary files in `source/resources/skills/`, copied automatically by packer and located through MyGo's resource path. Direct runs from `source/` use that same directory; distributing a bare executable alone does not include skills. Canvas runtime resources remain embedded in the Go binary.

## Development checks

From `source/`:

```sh
gofmt -l cmd internal
GOWORK=off go test ./...
GOWORK=off go test -race ./internal/...
GOWORK=off go vet ./...
GOWORK=off go build -o ../dist/locus ./cmd/locus
```

Formatting must produce no paths. Default tests do not open native windows or launch external Agents. MCP HTTP tests use temporary local listeners. Watch tests create temporary directories. Terminal tests validate launch arguments and popover behavior without launching shells or Agents.

On an explicitly available desktop session, run native tests:

```sh
LOCUS_NATIVE_SMOKE=1 GOWORK=off go test ./cmd/locus -run TestNative -count=1 -timeout=45s
```

Optionally set `LOCUS_SCREENSHOTS` to retain captures, including article backgrounds and metadata across reference titles, light/dark themes, and narrow/wide windows. Tests open a temporary Markdown library and a real MyGo WebView, checking template initialization, resource fetch, DOM interaction, settings retention, and release on departure. A terminal connection fixture also checks native terminal rendering and session cleanup without launching a shell or Agent. Surface captures do not establish OS material appearance or native browser overlay composition. Complete keyboard, pointer, accessibility, terminal authorization, and installation acceptance remains manual; do not launch external terminals or Agents automatically for it.

Documentation changes require size budgets, Subjects coverage, relative links and anchors, and changelog structure verification, followed by `git diff --check` from the repository root. Keep English and Chinese README content aligned. Report actual checks and platforms, not historical evidence from another implementation.

## Release notes

[CHANGELOG.md](../CHANGELOG.md) records user-visible pending outcomes and confirmed releases. The version in `source/packer.yaml` is not proof of publication. Before an explicitly authorized release, review scope, move completed shipping entries from `Unreleased` to the confirmed version, retain `Unreleased` and deferred entries, and synchronize packaging. Verify installation on every target platform before distribution.

There is no automated publication or automatic-update integration configured. Creating tags does not publish artifacts. Packaging does not authorize uploading releases or changing signing/update credentials. See [release boundary](architecture/packaging.md#release-boundary).
