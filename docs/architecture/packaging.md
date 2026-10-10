# App packaging

Locus uses Onevra's Go packager as a tool dependency in `source/go.mod`. `source/packer.yaml` owns product identity, version, entry point, target selection, and output location. The app is `Locus`, with identifier `me.oneo.locus`; the main package is `cmd/locus`.

## Inputs and ownership

Both Onevra and customized MyGo are top-level local replacements. The WebView branch's prepared MyGo checkout is required when building the app or tool. No project-owned native bridge or secondary build system is used. Check [dependency setup](../build.md#dependencies) before changing those replacements.

The packager owns native bundle layout, icon processing, signing behavior, and platform archive construction. Locus places `icon.icon`, `icon.ico`, and `icon.png` at the root of `source/resources/`; packer discovers them using its native naming rules and excludes icon sources from copied payloads. It automatically copies `resources/skills/` into the app resources without resource mappings or copy hooks. The desktop app resolves that directory through MyGo's `PathResources`, as Onevra Gallery does: bundle resources on macOS, the executable directory in portable packages, and the module's `resources/` directory for direct source runs. Unspecified packaging settings use packer defaults. Tailwind and its license are embedded by the canvas package.

The default configured archive target is Apple Silicon macOS. Outputs live beneath `dist/`, including versioned release archives. The `run` command builds and launches a development bundle; `build` creates the configured archives. VS Code tasks use the same commands. Direct `go run ./cmd/locus` is also a supported development path.

## Release boundary

Packaging is local assembly, not publication. No release script, tag-triggered workflow, update feed, Sparkle integration, or automatic publisher is configured. The app reports automatic updates unavailable and provides a manual releases link. Do not infer a published release from the configured version or from a successful archive build.

Before an explicitly authorized release, review shipping scope, move only completed shipping entries from `Unreleased` to the confirmed version in [CHANGELOG](../../CHANGELOG.md), retain `Unreleased`, synchronize the packaging version, and verify installation on every shipping architecture. Signing/notarization, distribution trust, and update delivery need an explicit release decision and separate acceptance; do not reuse signing keys or publish artifacts automatically.

## Verification

Run the [Go development checks](../build.md#development-checks), then `go tool packer build` in `source/`. Inspect the generated bundle's identity, version, icon, executable, and bundled skill resources. Development must not automatically launch external terminals or Agents.

macOS native reading and WebView tests are available. Other architectures, Windows builds, WebView2 prerequisites, Linux startup, installation, signing trust, and upgrades require their own environments. A macOS archive does not establish those outcomes.
