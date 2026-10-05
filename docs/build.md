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

Development bundles include the usual icons, resources, runtime dependencies, and signing. Resource loading does not fall back to the source tree; after changing bundled skills, rerun the packager to refresh the app resources. Their name gains ` Dev` and their Bundle Identifier gains `.dev`, leaving release outputs untouched. This does not isolate product storage: Locus still shares its existing settings, caches, library, and MCP port with the release app. Local development bundles do not include Sparkle.

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

## Manual macOS releases

There is no GitHub Actions release workflow; pushing a version tag does not build or publish an app. The retained `scripts/release.ts` can assemble and publish a universal Apple Silicon / Intel macOS ZIP, Sparkle update feed, and checksum when explicitly invoked by a maintainer. Windows distribution is not implemented by this script. See [macOS releases and updates](architecture/updates.md) for implementation and trust boundaries.

### Environment and signing

Use a Mac with Xcode and Command Line Tools, the Metal toolchain, Rust with both `aarch64-apple-darwin` and `x86_64-apple-darwin` targets, Bun, and the GitHub CLI (`gh`). Run from a checkout of the intended release commit. Provide these environment variables securely; repository Actions secrets are not automatically available to a local process:

| Name | Value |
| --- | --- |
| `GITHUB_REPOSITORY` | `oneo-me/locus`, matching the script's repository constant |
| `GITHUB_REF_NAME` | The release tag, such as `v0.2026.2` |
| `GH_TOKEN` | GitHub token with permission to publish releases in the repository |
| `SPARKLE_PUBLIC_KEY` | Base64 Ed25519 public key printed by Sparkle's `generate_keys` |
| `SPARKLE_PRIVATE_KEY` | Contents of the private key file exported with `generate_keys -x` |
| `MACOS_SIGNING_MODE` | Optional: `ad-hoc` (default) or `developer-id` |

Download [Sparkle 2.10.0](https://github.com/sparkle-project/Sparkle/releases/tag/2.10.0), unpack it, and run its tools locally:

```sh
./bin/generate_keys
./bin/generate_keys -x sparkle-private-key.txt
```

Back up the private key securely and remove the exported plaintext file from your working directory. Never commit signing keys. Keep the same update key across releases: losing it without Developer ID signing requires users to reinstall manually.

The default ad-hoc mode requires no Apple developer account and skips notarization. Sparkle archive signatures remain mandatory. On first installation, macOS may block opening the app; users who trust the release can allow it in System Settings → Privacy & Security after attempting to open it. Do not disable Gatekeeper globally.

For optional Developer ID distribution, set `MACOS_SIGNING_MODE=developer-id` and provide all of these environment variables:

| Name | Value |
| --- | --- |
| `APPLE_CERTIFICATE_P12` | Base64-encoded Developer ID Application certificate exported with its private key as a .p12 |
| `APPLE_CERTIFICATE_PASSWORD` | Password protecting the exported .p12 |
| `APPLE_SIGNING_IDENTITY` | Full signing identity, starting with `Developer ID Application:` |
| `APPLE_ID` | Apple developer account email for notarization |
| `APPLE_APP_PASSWORD` | Apple app-specific password, not the account password |
| `APPLE_TEAM_ID` | Apple developer team ID |

Encode the certificate with `base64 -i DeveloperID.p12 | tr -d '\n'`. Never commit certificates or private keys. This mode enables hardened runtime signing, notarization, ticket stapling, and Gatekeeper assessment. Missing credentials fail instead of downgrading to ad-hoc. Keep the same Sparkle keys and bundle identifier when switching modes.

### Publish a version

1. Review shipping scope and move those entries from `Unreleased` to the confirmed version section in `CHANGELOG.md`. Keep the `Unreleased` heading and any deferred changes.
2. Set `workspace.package.version` in `source/Cargo.toml`, synchronize inherited package versions in `source/Cargo.lock`, and run development checks. The tag, Cargo metadata, and changelog version must match.
3. Commit the release preparation, push the branch, and explicitly create and push the matching version tag. Tags alone do not publish anything.
4. Set the environment above and run `bun scripts/release.ts` from the repository root. This command publishes to GitHub, not just a local build. Maintainers must serialize runs; there is no workflow concurrency guard.
5. Inspect published assets and perform real installation and upgrade acceptance on macOS before recommending the release.

Missing credentials, mismatched keys/versions, absent release notes, and versions not newer than latest fail rather than publishing incomplete updates. Publication uses a draft until all assets are available. A failed unpublished run can reuse its draft; published releases are immutable.

Download the ZIP from [Releases](https://github.com/oneo-me/locus/releases/latest), extract it, and move `Locus.app` into Applications. Users of earlier builds without an updater must install an updater-enabled app manually once. Sparkle then checks and downloads updates in the background and handles installation/restart without deleting libraries or preferences. Ordinary source builds have no updater. Local tests do not establish publication or end-to-end upgrade acceptance.

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
