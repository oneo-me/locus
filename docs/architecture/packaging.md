# App packaging

Locus packages with MyGo's CLI, pinned as a Go tool in `source/go.mod`. `source/mygo.json` owns the name, identifier, version, entry point, icon, and output directory. The app is Locus, identifier `me.oneo.locus`, main package `cmd/locus`.

## Inputs and ownership

The app and the CLI use the published modules pinned in `source/go.mod` and `source/go.sum`. There is no local checkout and no project-owned native bridge. See [dependency setup](../build.md#dependencies).

`source/assets/icon.png` is the square icon MyGo uses for every platform. `source/assets/icon.icon` is the Icon Composer project. macOS compiles it with Xcode 26 or later; otherwise the build keeps the PNG and warns. `source/assets/icon.ico` is kept with those sources and is not a payload file. Icon sources stay outside `source/resources/`, because MyGo ships that whole directory.

`source/resources/skills/` is the runtime payload. The app loads it through MyGo's resource path: the bundle on macOS, the directory next to the executable on Windows and Linux, and `source/resources/` for `go run`. Canvas Tailwind stays embedded in the binary. Builds disable cgo and sign ad hoc. macOS targets require macOS 13.

`go tool mygo dev` launches a bundle named Locus Dev, identifier `me.oneo.locus.dev`, under `source/.mygo/dev`. The app still stores preferences in `~/.oneo/locus`.

## Outputs

From `source/`, name every target. A build without `-platform` produces only the host.

```sh
GOWORK=off go tool mygo dev
GOWORK=off go tool mygo build -platform darwin/arm64,darwin/amd64,windows/amd64,linux/amd64
```

Each target is a directory under `dist/`. The build does not open a file manager.

| Target | Artifacts |
| --- | --- |
| macOS | `Locus.app`, plus `Locus <version> <arch>.dmg` when the build runs on macOS |
| Windows | `Locus.exe` with resources beside it. `Locus Setup <version> <arch>.exe` only when `makensis` is on `PATH`; MyGo downloads NSIS only on Windows |
| Linux | `locus`, a `.desktop` file, `locus-<version>-linux-<arch>.tar.gz`, `locus_<version>_<arch>.deb`, and `install.sh` |

A macOS host can cross-compile Windows and Linux. A disk image needs macOS. Inspect identity, version, icon, executable, and the bundled skill before treating a directory as a release.

## Release boundary

Packaging is local assembly, not publication. No tag-triggered workflow, release script, update feed, or automatic publisher is configured. Build the targets above and upload those artifacts to a GitHub Release by hand. Keep the tag aligned with `version` in `source/mygo.json` and the MCP server version in `internal/discovery`.

Signing, notarization, and update delivery are unconfigured. The app reports automatic updates unavailable and links to the releases page. A successful build does not mean a release was published.

Before an explicitly authorized release, review what ships, move completed entries from `Unreleased` to that version in [CHANGELOG](../../CHANGELOG.md), keep `Unreleased`, synchronize both versions, and verify the build. Do not reuse signing keys, change update credentials, or publish automatically.

## Verification

Run the [Go development checks](../build.md#development-checks), then the build command above. Development must not launch external terminals or Agents.

macOS native reading and WebView tests are available. A cross-build shows that Windows and Linux compile. It does not verify WebView2, GTK 3, WebKitGTK, installation, signing trust, or upgrades. Those need their own environments.
