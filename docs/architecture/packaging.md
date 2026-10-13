# App packaging

Locus uses Onevra's Go packager as a tool dependency in `source/go.mod`. `source/packer.yaml` owns product identity, version, entry point, target selection, and output location. The app is `Locus`, with identifier `me.oneo.locus`; the main package is `cmd/locus`.

## Inputs and ownership

The app and packager use published module versions pinned in `source/go.mod` and `source/go.sum`, without local checkouts or a project-owned native bridge. See [dependency setup](../build.md#dependencies).

The packager owns native bundle layout, icon processing, signing behavior, and platform archive construction. Locus places `icon.icon`, `icon.ico`, and `icon.png` at the root of `source/resources/`; packer discovers them using its native naming rules and excludes icon sources from copied payloads. It automatically copies `resources/skills/` into the app resources without resource mappings or copy hooks. The desktop app resolves that directory through MyGo's `PathResources`, as Onevra Gallery does: bundle resources on macOS, the executable directory in portable packages, and the module's `resources/` directory for direct source runs. Unspecified packaging settings use packer defaults. Tailwind and its license are embedded by the canvas package.

The configured release targets are Apple Silicon and Intel macOS (`zip`), Windows x64 (`zip`), and Linux x64 (`tar.gz`). Archives are named `<slug>-<os>-<arch>-<version>` and live beneath `dist/releases/<version>/`. A macOS host produces every target: the packager cross-compiles Windows and Linux with cgo disabled while macOS alone compiles the Icon Composer icon and signs the bundle. The `run` command builds and launches a development bundle; `build` creates all configured archives, then opens `dist/releases/<version>/` in the system file manager only after a successful build. Failure to open the directory produces a warning without failing the build. VS Code tasks use the same commands. Direct `go run ./cmd/locus` is also a supported development path.

## Release boundary

Packaging is local assembly, not publication. The configured targets produce every platform's archive, but no tag-triggered workflow, release script, update feed, or automatic publisher is configured. Build with `go tool packer build` in `source/` and upload the archives from `dist/releases/<version>/` to a GitHub Release manually, keeping the release tag aligned with the `source/packer.yaml` version. Signing/notarization, distribution trust, and update delivery are also unconfigured: archives carry ad-hoc signatures, and the app reports automatic updates unavailable with a manual releases link. Do not infer a published release from the configured version or a successful archive build alone.

Before an explicitly authorized release, review shipping scope, move only completed shipping entries from `Unreleased` to the confirmed version in [CHANGELOG](../../CHANGELOG.md), retain `Unreleased`, synchronize the packaging version in `source/packer.yaml` and the MCP server version in `internal/discovery`, and verify the build. Do not reuse signing keys, change update credentials, or publish artifacts automatically.

## Verification

Run the [Go development checks](../build.md#development-checks), then `go tool packer build` in `source/`. The build produces one archive per configured target; inspect each archive's identity, version, icon, executable, and bundled skill resources. Development must not automatically launch external terminals or Agents.

macOS native reading and WebView tests are available. Cross-building verifies that the Windows and Linux targets compile and package; it does not verify WebView2 prerequisites, Linux startup (GTK 3 and WebKitGTK), installation, signing trust, or upgrades, which need their own environments. A macOS archive does not establish those outcomes.
