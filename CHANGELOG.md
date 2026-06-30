# Changelog

All notable changes to Dhara Storage are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **Granular `dhara_tool` CI commands** — `quality *`, `native merge`, and `package stage-native --msvc-env` replace removed shell wrappers.
- **`dhara-tool-build` workflow** — version-keyed Actions cache builds `profile.dist` binaries per OS; pipeline jobs restore cached tools instead of compiling each run.
- **Independent tool versioning** — `tooling/dhara_tool/Cargo.toml` owns tool semver; `[tool].version` in `dhara.config.toml` pins CI cache lookups (`config sync` keeps them aligned).
- **Local dist ensure scripts** — `ensure-dhara-tool-dist` builds `profile.dist` to `target/dist/` only when the binary is missing or `--version` ≠ manifest; VS Code tasks/launch split dev (`cargo run`) vs dist.

### Changed

- **Build profiles** — removed `[profile.ci]`; operator CLI uses `[profile.dist]` (optimized, rare rebuilds on tool version bump).
- **Pipeline** — PR/CD jobs invoke `target/dist/dhara_tool` subcommands; `verify-local` ensures dist then runs `quality run` (CI parity).
- **`dhara-tool-build`** — `cargo test -p dhara_tool` runs once on Linux; matrix legs only compile `profile.dist` per OS (binaries are not portable).
- **Linux-primary orchestration** — `quality`, `publish-readiness`, and CD `publish` run on `ubuntu-latest` with `linux-x64` tool cache; `platform-windows` remains on `windows-latest` for MSVC native DLL builds.
- **Operator output paths** — `dhara_tool` writes logs, scratch artifacts, and NuGet output under the executable directory (`{tool_root}/logs`, `{tool_root}/artifacts`, `{tool_root}/output`); e.g. `target/dist/logs/` when using the cached dist binary. Workspace sources (`filedefs.dat`, TrID package inputs) stay repo-relative.

### Removed

- **Staging/release shell scripts** — `merge-native`, `stage-native-*`, `verify-package`, and `release-run-windows` scripts deleted in favor of `dhara_tool` commands.

## [0.8.0] — 2026-06-30

Compared to [v0.7.1](https://github.com/D-Naveenz/dhara_storage/releases/tag/v0.7.1).

### Added

- **Cross-platform native asset staging** — CI and release workflows build and stage native libraries for all five 64-bit RIDs (`win-x64`, `win-arm64`, `linux-x64`, `linux-arm64`, `osx-arm64`).
- **Shell icon support** — OS shell icon RGBA pixels via `file_icon_provider`, exposed through file and directory information APIs and .NET bindings.
- **Unified GitHub Actions pipeline** — single `.github/workflows/pipeline.yml` for PR checks, platform tests, publish readiness, and release; replaces separate CI and release workflows.
- **Breaking changes policy** — `.cursor/rules/breaking-changes.mdc` documents pre-1.0 no-legacy, no-deprecation stance.
- **Documentation overhaul** — package-scoped READMEs, `docs/README.md` reference index, and expanded ABI, DSFD, CI/CD, and logging docs.

### Changed

- **Typed native ABI only** — removed legacy JSON ABI functions, DTOs, and `_json_old` entry points from `dharastorage` and .NET bindings.
- **CI script refactor** — GitHub Actions and local parity use standalone `tooling/scripts/` helpers; deprecated `verify ci`, `verify docs`, `release publish`, and `native merge` commands removed from `dhara_tool`.
- **Version bump** — workspace, crates, NuGet package, embedded `filedefs.dat` (`packageVersion` 0.8.0, revision 1), and shared config synchronized to `0.8.0`.

### Removed

- **Legacy JSON ABI** — all deprecated JSON serialization paths and related tests.

## [0.7.1] — 2026-06-29

Compared to [v0.7.0](https://github.com/D-Naveenz/dhara_storage/releases/tag/v0.7.0).

### Fixed

- **Crates.io release verification** — embed `filedefs.dat` from `src/core/dhara_storage_dal/resources/` so `cargo release` package verification succeeds outside the monorepo layout.
- **CI formatting** — rustfmt wrap fix in `dhara_tool` audit test imports.

### Changed

- **Default defs output** — `defs pack`, `build-trid-xml`, `sync-embedded`, and related commands now default to `src/core/dhara_storage_dal/resources/filedefs.dat`.
- **Default operator logs** — audit logs write to `tooling/logs/` instead of `tooling/output/logs/`; logs no longer follow `--output-dir`.
- **Version bump** — workspace, crates, NuGet package, and shared config synchronized to `0.7.1`.

## [0.7.0] — 2026-06-28

Compared to [v0.6.0](https://github.com/D-Naveenz/dhara_storage/releases/tag/v0.6.0) (2026-06-22).

### Added

- **DSFD definition packages** — `filedefs.dat` now uses the Dhara Storage File Definition (DSFD) container format (version 2): fixed header, FlatBuffers payload, and XML metadata footer. See [docs/filedefs-dat.md](docs/filedefs-dat.md).
- **FlatBuffers codec in `dhara_storage_dal`** — encode/decode pipeline, bundled definition loading, and container validation for DSFD packages.
- **DSFD metadata schema** — XSD schema (`schema/dsfd-metadata.xsd`) and XML footer parsing for package revision, tags, and definition counts.
- **Operator logging for `dhara_tool`** — structured audit logs with session/module lifecycle, phase timing, TrID transform stats, and subprocess milestones. Human reference: [docs/logging.md](docs/logging.md).
- **Logging CLI flags** — `-m` / `--min` (WARN-only file detail) and `-t` / `--trace` (DEBUG file detail including per-definition reduce trace).
- **Worker thread control** — `-w` / `--workers` caps Rayon parallelism for TrID parse/reduce (default 4); `TOOL_MAX_WORKERS` env support.
- **Flexible global options** — verbose and other global flags may appear before or after subcommands.
- **NuGet and release flows in `dhara_tool`** — packaging and publish capabilities previously in `dhara_storage_ops` are now part of the operator CLI.
- **Workspace state management** — package revisioning and embedded-def sync workflows in `dhara_tool`.
- **VS Code tasks** — build/verify task definitions in `.vscode/tasks.json`.
- **Git LFS tracking** — `.dat` artifacts tracked via Git LFS.

### Changed

- **Version bump** — workspace, crates, NuGet package, and shared config synchronized to `0.7.0`.
- **Directory layout** — runtime crates moved from `src/static/` to `src/core/`; C ABI crate moved from `src/dynamic/` to `src/dharastorage/`.
- **`filedefs.dat` location** — canonical runtime artifact is now `tooling/output/filedefs.dat` (embedded into `dhara_storage_dal` at compile time).
- **`dhara_tool` architecture** — consolidated command registry, filedefs/TrID modules, NuGet/release helpers, and capability routing; output staged under `tooling/output/` and `tooling/artifacts/`.
- **Definition package identifier** — on-disk magic and FlatBuffers layout migrated from legacy `FDEF` to `DSFD` format version 2.
- **Release workflow** — GitHub Actions release job updated for the new crate paths and tooling layout.
- **Documentation** — README, AGENTS.md, and crate READMEs updated for new paths, DSFD format, and logging conventions.

### Removed

- **`dhara_storage_ops` crate** — operator capabilities merged into `dhara_tool`; workspace and docs no longer reference the separate ops package.
- **Legacy FDEF container format** — version 1 packages with duplicate `DSFD`/`FDEF` markers are not supported.
- **`authors` fields** — removed from workspace `Cargo.toml` files to streamline package metadata.

### Migration notes

- **Custom `filedefs.dat` files** must be rebuilt with `dhara_tool` using the DSFD format. Packages produced for 0.6.x (`FDEF`) will not load in 0.7.0.
- **Import paths** — update any hard-coded references from `src/static/dhara_storage` or `src/dynamic/dharastorage` to `src/core/dhara_storage` and `src/dharastorage`.
- **Tooling commands** — replace `dhara_storage_ops`-based workflows with `cargo run -p dhara_tool -- …` equivalents (`verify ci`, `verify package`, `release run`, `defs sync-embedded`, etc.).

---

## [0.6.0] — 2026-06-22

Initial tagged release in this changelog series. See git history before `v0.6.0` for earlier changes.

[0.8.0]: https://github.com/D-Naveenz/dhara_storage/compare/v0.7.1...v0.8.0
[0.7.1]: https://github.com/D-Naveenz/dhara_storage/compare/v0.7.0...v0.7.1
[0.7.0]: https://github.com/D-Naveenz/dhara_storage/compare/v0.6.0...v0.7.0
[0.6.0]: https://github.com/D-Naveenz/dhara_storage/releases/tag/v0.6.0
