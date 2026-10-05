# Running and building

Install Rust and the appropriate platform build environment:

- macOS: Xcode and Command Line Tools; macOS 13 or later.
- Windows: Visual Studio C++ tools and the Windows SDK.

Code, the Cargo workspace, resources, icons, and packaging configuration live in `source/`. Final app outputs live in the root `dist/` directory, which Git ignores. Outputs are not split into platform or architecture subdirectories. See [AGENTS.md](../AGENTS.md) for development rules and the [Architecture overview](../ARCHITECTURE.md) for module and directory responsibilities.

Run from the source directory:

```sh
cd source
cargo run --locked -p oneo-tool-packer -- run
```

The `run` command builds with Cargo's development profile, assembles `dist/Locus Dev.app` on macOS or `dist/Locus Dev/` on Windows, and launches the bundled executable in the foreground. Logs stay in the terminal; on macOS, Ctrl+C and exit status behave like running the executable directly. Arguments after `--` are forwarded to the app. `run --dry-run` previews the build and launch without executing either. Quit the development app before rebuilding it; the tool does not terminate existing instances.

Development bundles include the usual icons, resources, runtime dependencies, and signing. Their name gains ` Dev` and their Bundle Identifier gains `.dev`, leaving release outputs untouched. This does not isolate product storage: Locus still shares its existing settings, caches, library, and MCP port with the release app. Local development bundles do not include Sparkle.

## Building the app

App versions follow `0.<year>.<release number within the year>`. Release numbers start at 1, increase within a year, and reset in a new year. The version is maintained in `workspace.package.version` in `source/Cargo.toml` and read by the packager. When changing it, synchronize workspace packages inheriting that version in `source/Cargo.lock`.

From the repository root:

```sh
cargo run --manifest-path source/Cargo.toml --locked -p oneo-tool-packer -- --project source
```

Build configuration lives in [source/packer.toml](../source/packer.toml). The default builds Release for the current platform and saves to `dist/`: `dist/Locus.app` on macOS, or `dist/Locus/` containing the executable and runtime dependencies on Windows. `output-directory = "../dist"` resolves relative to the source directory containing the packaging configuration. A later successful packaging run replaces an output with the same name.

VS Code's `dev` and `build` tasks run in `source/`. Both tasks invoke the repository's packager through Cargo; `dev` adds `run`, while `build` retains release packaging. To install the tool separately, run `cargo install --path source/crates/oneo-tool-packer --locked`, then `oneo-tool-packer run --project source` to develop or `oneo-tool-packer --project source` to package Release from the root.

The build directory follows Cargo's global or project `build.target-dir` configuration and the `CARGO_TARGET_DIR` environment variable. The packager reads the actual executable path from Cargo build messages and supports shared build directories without a path setting in `packer.toml`. Final bundles still go to the project's `dist/`. Resolution and precedence follow [Cargo configuration rules](https://doc.rust-lang.org/cargo/reference/config.html#buildtarget-dir).

macOS uses ad-hoc signing by default. Windows builds must run in a Windows environment.

## Release notes

The root [CHANGELOG.md](../CHANGELOG.md) records completed pending changes and confirmed releases for Locus desktop users. The packager reads versions from Cargo metadata and assembles the app; it does not read the changelog or generate release notes. Maintainers update release notes alongside changes.

Record routine user-visible changes under `Unreleased`. Do not promise planned or unfinished capabilities. Before release, check the actual branch and platform outputs, move shipping entries into the confirmed version section, and retain the top `Unreleased` heading and any deferred entries. Do not infer historical releases from the current Cargo version or commit history.

After confirming a version, synchronize `source/Cargo.toml` and `source/Cargo.lock` as above, build, and complete target-platform checks. Routine documentation maintenance does not trigger version bumps, tags, or publishing.

## Automated macOS releases

The [release workflow](../.github/workflows/release.yml) runs when a version tag such as `v0.2026.2` is pushed. It publishes a universal Apple Silicon / Intel macOS ZIP, an update feed, and a checksum to GitHub Releases. Windows is not released or updated by this workflow. The implementation and trust boundaries are described in [macOS releases and updates](architecture/updates.md).

### Default distribution without an Apple account

The default mode uses ad-hoc code signing and skips Apple notarization. No Apple developer membership or Apple credentials are required. Sparkle archive signatures are independent of Apple code signing and remain mandatory. Configure only these two values under GitHub repository Settings → Secrets and variables → Actions:

| Type | Name | Value |
| --- | --- | --- |
| Variable | `SPARKLE_PUBLIC_KEY` | Base64 Ed25519 public key printed by Sparkle's `generate_keys` |
| Secret | `SPARKLE_PRIVATE_KEY` | Contents of the private key file exported with `generate_keys -x` |

Download [Sparkle 2.10.0](https://github.com/sparkle-project/Sparkle/releases/tag/2.10.0), unpack it, and run its tools locally:

```sh
./bin/generate_keys
./bin/generate_keys -x sparkle-private-key.txt
```

Copy the public key into the variable and the exported file's contents into the private-key secret. Back up the private key securely and remove the exported plaintext file from your working directory. Never commit signing keys. Keep the same update key across releases: losing it without Developer ID signing requires users to reinstall manually.

Leave `MACOS_SIGNING_MODE` unset or set the repository variable to `ad-hoc`. This produces an app that is not Apple-notarized. On first installation, macOS may block opening it; after attempting to open it, users who trust the release can allow it in System Settings → Privacy & Security. Do not disable Gatekeeper globally. Sparkle verifies update authenticity, but does not make an app Apple-notarized or guarantee identical OS authorization behavior across macOS versions.

### Optional Developer ID distribution

If you later join Apple's paid developer program, explicitly set the repository variable `MACOS_SIGNING_MODE` to `developer-id` and configure all of these secrets:

| Type | Name | Value |
| --- | --- | --- |
| Secret | `APPLE_CERTIFICATE_P12` | Base64-encoded Developer ID Application certificate exported with its private key as a .p12 |
| Secret | `APPLE_CERTIFICATE_PASSWORD` | Password protecting the exported .p12 |
| Secret | `APPLE_SIGNING_IDENTITY` | Full signing identity, starting with `Developer ID Application:` |
| Secret | `APPLE_ID` | Apple developer account email for notarization |
| Secret | `APPLE_APP_PASSWORD` | Apple app-specific password, not the account password |
| Secret | `APPLE_TEAM_ID` | Apple developer team ID |

Encode the exported certificate with `base64 -i DeveloperID.p12 | tr -d '\n'` for `APPLE_CERTIFICATE_P12`. Never commit Apple certificates or private keys. This mode enables hardened runtime signing, notarization, ticket stapling, and Gatekeeper assessment. Missing Apple credentials in this explicitly selected mode fail the run instead of downgrading to ad-hoc. Keep the same Sparkle keys and bundle identifier when switching modes.

### Publish a version

For each release:

1. Review shipping scope and move those entries from `Unreleased` to a confirmed `## [0.<year>.<release>]` section in `CHANGELOG.md`. Keep the `Unreleased` heading and any deferred changes.
2. Set `workspace.package.version` in `source/Cargo.toml`, synchronize inherited workspace package versions in `source/Cargo.lock`, and run development checks. The tag, Cargo metadata, and changelog version must match.
3. Commit the release preparation and push the branch. Then create and push its version tag:

   ```sh
   git tag v0.2026.2
   git push origin v0.2026.2
   ```

4. Inspect the Actions run and published assets. On a failed unpublished run, fix the external configuration and rerun; an existing draft is reused. Code changes require a new release commit/tag, not silently moving a published tag.
5. Perform real installation and upgrade acceptance on macOS before recommending the release to users.

The workflow needs Actions enabled and permission to write repository contents. It uses the workflow's `GITHUB_TOKEN`; no separate personal access token is needed. Missing Sparkle keys, missing Apple credentials in Developer ID mode, mismatched keys/versions, absent release notes, and versions not newer than latest fail rather than publishing unauthenticated or incomplete updates. Publication uses a draft until all assets are available.

Download the ZIP from [Releases](https://github.com/oneo-me/locus/releases/latest), extract it, and move `Locus.app` into Applications. This first updater-enabled app must be installed manually by users of earlier builds. Subsequently Sparkle checks and downloads updates in the background and handles installation/restart through its native flow without deleting libraries or preferences. Source builds have no updater unless assembled by the release pipeline. No GitHub release or end-to-end upgrade is verified merely by local unit tests.

## Development checks

Run Cargo commands in `source/`, selecting formatting, package tests, and Clippy according to the change's impact:

```sh
cargo fmt --all --check
cargo test -p <package> --locked
cargo clippy -p <package> --all-targets --all-features --locked -- -D warnings
```

For release pipeline changes, also run `bun test scripts/release.test.ts` from the repository root. For changes spanning packages, check all affected packages. Global dependencies or shared interfaces may require workspace checks. Default `cargo test` selects only the default app package and does not establish that feature packages were tested. MCP HTTP regressions require local temporary ports; see [Agent integration](architecture/mcp.md#verification-methods). Run Node canvas tests from the repository root as described in [Canvas](architecture/canvas.md#verification-boundaries). Report actual checks, platforms covered, and unverified items. Compilation and headless tests do not replace acceptance testing of windows, reading, terminals, settings, system authorization, or native interaction.

Use the locked GPUI Kit version's public APIs and verify real signatures before implementation. Organize modules by capability and coordinate through public events, methods, and data types. Feature packages must not access sibling package internals, and `oneo-gpui*` remains product-neutral. See [Settings and persistence](architecture/settings.md) for runtime configuration and localization ownership and [Desktop workspace](architecture/workspace.md#interface-and-state-conventions) for interface conventions.

Write project documentation and code comments in English, except for explicitly maintained translations such as `README.zh-CN.md`. Localized interface strings and multilingual test fixtures retain their intended languages.

New development scripts use Bun and TypeScript and live in the root `scripts/` directory. If introducing a `package.json` manifest, expose script commands there and register actual `workspaces` paths for any JavaScript/TypeScript subpackages. See [AGENTS.md](../AGENTS.md#verification) for documentation verification requirements. Check Markdown in the repository root and under `docs/` for size budgets, Subjects coverage, relative links, heading anchors, and Unreleased structure. Also verify targets and anchors when linking to source or bundled Locus skills. Review mechanism coverage, agreement between documentation and implementation, and release-note accuracy.
