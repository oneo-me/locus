# macOS releases and updates

## Ownership and scope

`locus-updater` owns an app-wide GPUI entity, a main-thread Sparkle standard updater controller, and its retained delegate. The app initializes the shared entity during startup; Settings observes it through the package's public API. Sparkle owns scheduling, native update UI, archive verification, installation, authorization, and relaunch. The shared GPUI shell and document library do not implement updating. Windows has no updater and is not published by the release script.

There is no GitHub Actions release workflow; pushing tags does not publish releases. Maintainers explicitly invoke the retained release script with a release environment. `scripts/release.ts` validates the exact stable `v0.<year>.<release number>` form, compares it with the locked Cargo metadata version, and requires release notes for that version. Tags must be newer than the published latest release; existing published versions are immutable. Forks must explicitly change the script's repository constant before publishing their own feed.

## Release assembly and publication

A macOS runner with Xcode and both Rust macOS targets builds the existing packager twice. The script combines the two executables with `lipo`, verifies both slices, and retains one app's common resources. The universal bundle supports Apple Silicon and Intel on macOS 13 or later.

The script downloads Sparkle 2.10.0, verifies the pinned SHA-256, and embeds `Sparkle.framework` under `Contents/Frameworks`. This step is specific to Locus distribution; the generic packager and ordinary local builds do not download or embed an updater. Before signing, the script adds these Info.plist defaults:

- `SUFeedURL`: `https://github.com/oneo-me/locus/releases/latest/download/appcast.xml`.
- `SUPublicEDKey`: the configured Ed25519 public key.
- `SUVerifyUpdateBeforeExtraction`: true, requiring archive authentication before extraction.
- `SUEnableAutomaticChecks` and `SUAutomaticallyUpdate`: true.
- `SUScheduledCheckInterval`: 86400 seconds.

`MACOS_SIGNING_MODE` defaults to `ad-hoc`. This mode needs no Apple developer account or Apple credentials. Nested Sparkle XPC services, helper executable, updater app, framework, and finally Locus are signed inside-out with the local ad-hoc identity. Hardened runtime flags are explicitly cleared to avoid library validation requiring an Apple team identity. Strict deep verification checks bundle integrity, but does not establish publisher identity or Gatekeeper acceptance. Notarization, stapling, and Gatekeeper assessment are skipped. The app is not Apple-notarized; the release body includes a first-installation warning.

Setting the mode explicitly to `developer-id` requires all Apple credentials. In this mode, a temporary keychain imports the Developer ID Application certificate. Inside-out signing uses hardened runtime and trusted timestamps. The ZIP is submitted to Apple's notarization service, and the app's ticket is stapled and validated. Gatekeeper assessment must succeed. The script recreates the final ZIP after stapling. Invalid modes and incomplete Developer ID configuration fail rather than silently falling back to ad-hoc.

In both modes, the script signs the final ZIP bytes with Sparkle's private key and independently verifies the signature against the public key embedded in the app. A mismatched key pair aborts publication. Ad-hoc code signing is not the update archive's authentication mechanism; the Ed25519 key is.

Release assets are:

- `Locus-<version>-macos-universal.zip`: the ad-hoc app by default, or Developer ID signed and notarized app in that mode; also used for updates.
- `appcast.xml`: one stable version with minimum macOS version, immutable tag-specific archive URL, archive length, and EdDSA signature.
- `SHA256SUMS`: checksum for manual download verification, not the updater's trust mechanism.

The script creates or resumes a draft Release, uploads all assets, and only then publishes it as latest. Maintainers must serialize invocations; the script has no concurrency guard. Failed build, signing, notarization, or upload does not change the public latest feed. A rerun can replace assets on an unpublished draft; it cannot modify a published release. In Developer ID mode, temporary certificate files and the keychain are removed in a finally block, and the previous keychain search list is restored. Ad-hoc mode does not import a certificate or alter keychains. Release bodies use the matching version section in `CHANGELOG.md`, never `Unreleased` or generated commit lists; ad-hoc mode prepends an installation warning.

## Installed application behavior

During startup, the macOS updater service loads Sparkle from its own bundle and retains `SPUStandardUpdaterController` and a weakly referenced native updater delegate on the main thread. Explicit `startUpdater:` initialization returns configuration failures to the entity instead of silently losing them. Missing frameworks and unsupported platforms produce an unavailable state; loading or configuration failures produce an error state.

Settings → About displays application information and the service's current result. Application-menu About navigates there without checking; Check for Updates navigates there and defers the native check. App-wide action fallbacks activate the retained main window. The in-page Button invokes the same service operation, guards duplicate checks, and observes native readiness. Source runs have a disabled check Button with an explanation and a manual GitHub releases link.

The native delegate sends check-start, new-version, no-update reason, and completion events through an async channel, including checks scheduled automatically by Sparkle. A retained foreground task applies them to the GPUI entity and notifies observers, without polling, updating GPUI from native callbacks, or accessing settings internals. No-update presentation prefers NSError's detailed recovery/failure text over a generic title so incompatible OS versions are not treated as evidence of being current. Cancellation returns a pending check to ready rather than claiming success. Results survive navigating away from About for the process lifetime; only Sparkle owns persisted scheduling preferences.

Distributed builds enable automatic checking and background downloading by default. Sparkle retains its own user-default preferences rather than rewriting them on each launch. Its native update flow handles installation and restart, including deferral and filesystem authorization; Locus does not force an immediate restart when a tag appears. Offline checks do not prevent normal library use. The public feed exposes only the current stable Release; there are no prerelease channels or delta updates.

HTTPS protects feed transport. The immutable ZIP must pass Sparkle's EdDSA verification before extraction and installation. Sparkle applies its code-signing policy as well when Developer ID signing is available; an ad-hoc identity is not an Apple-verified publisher. Do not rotate or lose the Ed25519 key casually: installed apps trust the public key they shipped with. Without Developer ID signing, losing the update key requires users to manually reinstall a trusted release. Retain secure backups and follow [Sparkle's key migration guidance](https://sparkle-project.org/documentation/) for any future rotation. Keep the bundle identifier and update key consistent across releases and signing modes, and retain a consistent Developer ID identity when using that mode.

Users running versions without this updater must manually install the first updater-enabled Release once. Install the extracted app in `/Applications` or a writable user Applications directory, rather than running it inside an archive. Local settings and libraries are outside the replaced app bundle and are not removed during updates. On first installation of an ad-hoc build, Gatekeeper may block opening the app. Only after trusting the release, users can attempt to open it and confirm the exception in System Settings → Privacy & Security. Do not disable Gatekeeper globally. Sparkle archive authentication does not replace Apple notarization or guarantee identical OS authorization behavior on every macOS version.

## Configuration and verification

See [Manual macOS releases](../build.md#manual-macos-releases) for environment variables, credentials, and the maintainer procedure. This requires a public GitHub repository/feed; private authenticated feeds are not implemented.

Run `bun test scripts/release.test.ts`, tests and Clippy for `locus-updater`, `locus-settings`, `locus-workspace`, and `locus`, and `cargo fmt --all --check`. Updater tests cover shared entity identity, unavailable builds, native result transitions, and cancellation; settings/workspace UI integration tests cover navigation and command routing. Release-contract tests cover signing-mode defaults and required credentials, inside-out signing command options, tag/version agreement, numeric ordering, release-note extraction, appcast enclosure validation, archive tampering, and incorrect key pairs. They do not substitute for a hosted release job or a native installation test.

Before production distribution, invoke the release script with configured Sparkle keys and, only for Developer ID mode, Apple credentials. On real Apple Silicon and Intel Macs, install an updater-enabled older app and publish a newer version. Confirm background discovery, valid signature installation, deferred installation/restart, preserved library and preferences, offline behavior, and rejection of a modified archive. Also inspect ZIP contents, universal executable slices, and plist defaults. For ad-hoc releases, verify the first-installation Gatekeeper exception flow and subsequent updates on supported macOS versions. For Developer ID releases, verify signing, notarization, and Gatekeeper acceptance. Local compilation, headless UI integration tests, and native menu/check-result smoke tests cannot establish complete download/install/relaunch acceptance.
