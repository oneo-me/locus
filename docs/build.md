# Running and building

Install Rust and the appropriate platform build environment:

- macOS: Xcode and Command Line Tools; macOS 13 or later.
- Windows: Visual Studio C++ tools and the Windows SDK.

Code, the Cargo workspace, resources, icons, and packaging configuration live in `source/`. Final app outputs live in the root `dist/` directory, which Git ignores. Outputs are not split into platform or architecture subdirectories. See [AGENTS.md](../AGENTS.md) for development rules and the [Architecture overview](../ARCHITECTURE.md) for module and directory responsibilities.

Run from the source directory:

```sh
cd source
cargo run
```

## Building the app

App versions follow `0.<year>.<release number within the year>`. Release numbers start at 1, increase within a year, and reset in a new year. The version is maintained in `workspace.package.version` in `source/Cargo.toml` and read by the packager. When changing it, synchronize workspace packages inheriting that version in `source/Cargo.lock`.

From the repository root:

```sh
cargo run --manifest-path source/Cargo.toml --locked -p oneo-tool-packer -- --project source
```

Build configuration lives in [source/packer.toml](../source/packer.toml). The default builds Release for the current platform and saves to `dist/`: `dist/Locus.app` on macOS, or `dist/Locus/` containing the executable and runtime dependencies on Windows. `output-directory = "../dist"` resolves relative to the source directory containing the packaging configuration. A later successful packaging run replaces an output with the same name.

VS Code's `dev` and `build` tasks run in `source/`. The build task invokes the repository's packager through Cargo. To install the tool separately, run `cargo install --path source/crates/oneo-tool-packer --locked`, then `oneo-tool-packer --project source` from the root.

The build directory follows Cargo's global or project `build.target-dir` configuration and the `CARGO_TARGET_DIR` environment variable. The packager reads the actual executable path from Cargo build messages and supports shared build directories without a path setting in `packer.toml`. Final bundles still go to the project's `dist/`. Resolution and precedence follow [Cargo configuration rules](https://doc.rust-lang.org/cargo/reference/config.html#buildtarget-dir).

macOS uses ad-hoc signing by default. Windows builds must run in a Windows environment.

## Release notes

The root [CHANGELOG.md](../CHANGELOG.md) records completed pending changes and confirmed releases for Locus desktop users. The packager reads versions from Cargo metadata and assembles the app; it does not read the changelog or generate release notes. Maintainers update release notes alongside changes.

Record routine user-visible changes under `Unreleased`. Do not promise planned or unfinished capabilities. Before release, check the actual branch and platform outputs, move shipping entries into the confirmed version section, and retain the top `Unreleased` heading and any deferred entries. Do not infer historical releases from the current Cargo version or commit history.

After confirming a version, synchronize `source/Cargo.toml` and `source/Cargo.lock` as above, build, and complete target-platform checks. Routine documentation maintenance does not trigger version bumps, tags, or publishing.

## Development checks

Run Cargo commands in `source/`, selecting formatting, package tests, and Clippy according to the change's impact:

```sh
cargo fmt --all --check
cargo test -p <package> --locked
cargo clippy -p <package> --all-targets --all-features --locked -- -D warnings
```

For changes spanning packages, check all affected packages. Global dependencies or shared interfaces may require workspace checks. Default `cargo test` selects only the default app package and does not establish that feature packages were tested. MCP HTTP regressions require local temporary ports; see [Agent integration](architecture/mcp.md#verification-methods). Run Node canvas tests from the repository root as described in [Canvas](architecture/canvas.md#verification-boundaries). Report actual checks, platforms covered, and unverified items. Compilation and headless tests do not replace acceptance testing of windows, reading, terminals, settings, system authorization, or native interaction.

Use the locked GPUI Kit version's public APIs and verify real signatures before implementation. Organize modules by capability and coordinate through public events, methods, and data types. Feature packages must not access sibling package internals, and `oneo-gpui*` remains product-neutral. See [Settings and persistence](architecture/settings.md) for runtime configuration and localization ownership and [Desktop workspace](architecture/workspace.md#interface-and-state-conventions) for interface conventions.

Write project documentation and code comments in English, except for explicitly maintained translations such as `README.zh-CN.md`. Localized interface strings and multilingual test fixtures retain their intended languages.

New development scripts use Bun and TypeScript and live in the root `scripts/` directory. If introducing a `package.json` manifest, expose script commands there and register actual `workspaces` paths for any JavaScript/TypeScript subpackages. See [AGENTS.md](../AGENTS.md#verification) for documentation verification requirements. Check Markdown in the repository root and under `docs/` for size budgets, Subjects coverage, relative links, heading anchors, and Unreleased structure. Also verify targets and anchors when linking to source or bundled Locus skills. Review mechanism coverage, agreement between documentation and implementation, and release-note accuracy.
