# App packaging

Locus packages with MyGo's CLI, pinned as a Go tool in `source/go.mod`. `source/mygo.json` owns the name, identifier, version, entry point, icon, and output directory. The app is Locus, identifier `me.oneo.locus`, main package `cmd/locus`.

## Inputs and ownership

The app and the CLI use the published modules pinned in `source/go.mod` and `source/go.sum`. There is no local checkout and no project-owned native bridge. See [dependency setup](../build.md#dependencies).

`source/assets/icon.png` is the square icon MyGo uses for every platform. `source/assets/icon.icon` is the Icon Composer project. macOS compiles it with Xcode 26 or later; otherwise the build keeps the PNG and warns. `source/assets/icon.ico` is kept with those sources and is not a payload file. Icon sources stay outside `source/resources/`, because MyGo ships that whole directory.

`source/resources/skills/` is the runtime payload. The app loads it through MyGo's resource path: the bundle on macOS, the directory next to the executable on Windows and Linux, and `source/resources/` for `go run`. Canvas Tailwind stays embedded in the binary. Builds disable cgo and sign ad hoc. macOS targets require macOS 13.

`go tool mygo dev` launches a bundle named Locus Dev, identifier `me.oneo.locus.dev`, under `source/.mygo/dev`. The app still stores preferences in `~/.locus`.

## Outputs

From `source/`, name every target. A build without `-platform` produces only the host.

```sh
GOWORK=off go tool mygo dev
GOWORK=off go tool mygo build -platform darwin/arm64,darwin/amd64,windows/amd64,windows/arm64,linux/amd64,linux/arm64
```

Each target is a directory under `dist/`. The build does not open a file manager.

| Target | Artifacts |
| --- | --- |
| macOS | `Locus.app`, plus `Locus <version> <arch>.dmg` when the build runs on macOS |
| Windows | `Locus.exe` with resources beside it. `Locus Setup <version> <arch>.exe` only when `makensis` is on `PATH`; MyGo downloads NSIS only on Windows |
| Linux | `locus`, a `.desktop` file, `locus-<version>-linux-<arch>.tar.gz`, `locus_<version>_<arch>.deb`, and `install.sh` |

A macOS host can cross-compile Windows and Linux. A disk image needs macOS. Inspect identity, version, icon, executable, and the bundled skill before treating a directory as a release.

## Release

Pushing a tag `v` plus the `version` in `source/mygo.json` runs [release.yml](../../.github/workflows/release.yml). The workflow checks that tag against the version and the matching [CHANGELOG](../../CHANGELOG.md) section, then builds all six published targets: `darwin/arm64` and `darwin/amd64` on macOS, and `windows/amd64`, `windows/arm64`, `linux/amd64` and `linux/arm64` on Ubuntu. The draft release takes that changelog section as its notes. Each job uploads the installers and signed update files of its targets to that draft, and the last job publishes it. The Windows job installs NSIS, so the release includes both installers, one per architecture.

`updates` in `source/mygo.json` names `oneo-me/locus` and the public update key. The repository secret `MYGO_UPDATER_PRIVATE_KEY` signs the archives and manifests. Keep that secret key out of the repository. The workflow stops before publishing when the secret or the changelog section is missing.

Packaged builds embed the public key and install a newer signed release from that repository, by delta when one exists for the running version. Settings > About drives the check, download, and relaunch through `mygo.Updater`; see [About and updates](settings.md#about-and-updates). Development builds, the `.deb` in `/opt`, and other locations the app cannot write leave updates disabled. macOS Developer ID signing, notarization, and Windows Authenticode are not configured, so Gatekeeper and SmartScreen can still block a published build.

Before tagging, move completed entries from `Unreleased` to that version, keep `Unreleased`, and use the same version for the MCP server in `internal/discovery`.

## Verification

Run the [Go development checks](../build.md#development-checks), then the build command above. Development must not launch external terminals or Agents.

macOS native reading and WebView tests are available. A cross-build shows that Windows and Linux compile. It does not verify WebView2, GTK 3, WebKitGTK, installation, signing trust, or upgrades. Those need their own environments.
