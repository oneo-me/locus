# Running and building

Locus has one Go module under `source/` and uses Onevra's Go packager. No Rust toolchain is required or used. Read the [architecture overview](../ARCHITECTURE.md) and [AGENTS](../AGENTS.md) before changing mechanisms.

## Dependencies

Use Go 1.27.1 or later and a native desktop session. `source/go.mod` pins published Onevra `v0.2026.8` and MyGo `v0.4.8`; `source/go.sum` records their checksums. WebView uses MyGo's built-in API, so no local replacements, patched checkouts, or preparation scripts are needed.

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

On macOS, these development runs exit when the main window closes so they leave no Dock activity; packaged builds keep the retained window instead. The library can also be chosen through Open library. Existing preferences load from `~/.oneo/locus/settings.yaml`. Settings and note operations affect real local data; development uses the same product storage unless a test supplies isolated fixtures. Tests do not operate on the user's library.

The app watches external file changes. Canvas scripts rerun only after explicit refresh or reopening. Native canvases use a custom protocol; the app does not start an HTML development server or install Node dependencies.

## Building the app

From `source/`:

```sh
GOWORK=off go tool packer run
GOWORK=off go tool packer build
```

VS Code `run` and `build` tasks invoke these commands. [source/packer.yaml](../source/packer.yaml) targets Apple Silicon and Intel macOS (`zip`), Windows x64 (`zip`), and Linux x64 (`tar.gz`), and outputs to `dist/`. The application identity is Locus. Packer cross-compiles the Windows and Linux targets from the macOS host and produces one archive per target. After a successful build, the packager automatically opens `dist/releases/<version>/` containing the final archives in the system file manager. Set `openReleases: false` in `source/packer.yaml` to disable this for unattended builds. Verify native behavior on each target platform before claiming support.

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
LOCUS_NATIVE_SMOKE=1 GOWORK=off go test ./cmd/locus -run TestNative -count=1 -timeout=90s
```

On macOS these tests run in the background: the test app uses the accessory activation policy (no Dock icon or activation), windows are created hidden, and the harness asks hidden windows to display so input and captures receive fresh frames. The reading test fails if a window becomes visible or focused or the app activates. Set `LOCUS_NATIVE_FOREGROUND=1` to show windows instead; the close-and-reopen lifecycle test shows its window and runs only in that mode. Other platforms keep visible windows until hidden frame delivery is verified there.

Optionally set `LOCUS_SCREENSHOTS` to retain captures, including article backgrounds and metadata across reference titles, light/dark themes, and narrow/wide windows. Tests open a temporary Markdown library and a real MyGo WebView, checking template initialization, resource fetch, DOM interaction, settings retention, and release on departure. A terminal connection fixture also checks native terminal rendering and session cleanup without launching a shell or Agent. Surface captures do not establish OS material appearance or native browser overlay composition. Complete keyboard, pointer, accessibility, terminal authorization, and installation acceptance remains manual; do not launch external terminals or Agents automatically for it.

Documentation changes require size budgets, Subjects coverage, relative links and anchors, and changelog structure verification, followed by `git diff --check` from the repository root. Keep English and Chinese README content aligned. Report actual checks and platforms, not historical evidence from another implementation.

## Release notes

[CHANGELOG.md](../CHANGELOG.md) records user-visible pending outcomes and confirmed releases. The version in `source/packer.yaml` is not proof of publication. Before an authorized release, review scope, move completed shipping entries from `Unreleased` to the confirmed version, retain `Unreleased` and deferred entries, and synchronize the packaging version in `source/packer.yaml` and the MCP server version in `internal/discovery`.

Publication is manual: build every configured target with `go tool packer build` and upload the archives from `dist/releases/<version>/` to a GitHub Release whose tag matches the packaged version. No tag-triggered workflow, release script, or automatic publisher is configured, and automatic updates remain unavailable. Verify installation on every target platform before distribution. See [release boundary](architecture/packaging.md#release-boundary).
