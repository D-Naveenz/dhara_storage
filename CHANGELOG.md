# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### Added
- Added engine/interop `open_for_read` / `open_for_write` with `FileShareMode`, optional `lock_timeout` (retry only on sharing/busy), and OS access errors mapped to `StorageError` (including `LockTimeout`).
- Added public awaitable `StorageProcess` (Rust `Future` / .NET `Completion` + `GetAwaiter`) as the copy/move executioner; sync APIs wait inside and return void / `Result<()>`.
- Added `ProcessSession` in `dhara_storage_core::process` for engine cancel/events/`TaskQueue` helpers (session is not the public awaitable handle).
- Added bounded `TaskQueue` for long-running transfer sessions (single consumer over discrete path/size tasks).
- Added `StorageProcessEvent` progress model (`Started` / `CurrentItem` / `Bytes` / `Completed` / `Failed` / `Cancelled`) across Rust, `dhara-sd` gRPC streams, and .NET `IProgress<StorageProcessEvent>`.
- Documented deferred transfer work (RAM read-ahead pool, verify, broader process-first ops) in `docs/transfer-pipeline-next.md`.

### Changed
- Daemon `OpenReadHandle` / `OpenWriteHandle` open via the runtime APIs then dup/FD-pass; proto carries optional `ShareMode` and `lock_timeout_ms` (0 = fail immediately on busy).
- Write-handle default share is **exclusive** (clean-cut vs prior share-all daemon write); read-handle default remains shared.
- Copy/move are process-first: work starts immediately; `CopyAsync` / `start_*` return `StorageProcess`; sync `Copy` / `copy_*` wait and return void / `Result<()>`.
- Runtime copy/move transfers enqueue prepared tasks on a `TaskQueue` and write with a single consumer to avoid destination write thrashing.
- Replaced snapshot `StorageProgress` / `ProgressReporter` with `StorageProcessEvent` / `ProcessEventReporter` (clean-cut on managed APIs and daemon stream messages).
- Removed analyze-during-copy (`TransferOptions.analyze_content`); content intelligence stays on analyze/metadata APIs only.
- Removed thin Tokio `spawn_blocking` wrappers for copy/move (await `StorageProcess` instead).

### Removed
- Sync copy/move no longer return destination handles/`PathBuf` from the public sugar APIs (use `start_*` / `ProcessOutcome` / await destination when needed).

## v0.9.24 - 2026-08-10

### Changed
- Expanded `dhara_storage_core` beyond DSFD: shared process primitives (progress, cancellation, reporters) and portable types (transfer/read/write options, size, attributes, permissions).
- Runtime keeps concrete path handles and filesystem I/O; API-facing core types remain re-exported from `dhara_storage` for the usual app dependency path.

### Technical
- Bumped workspace / NuGet product line to **0.9.24**.

## v0.9.23 - 2026-08-08

### Changed
- Single `drot` binary for CLI and TUI (pinned DROT **0.10.0**): no subcommand on a TTY opens the TUI; subcommands and `--help` use the Direct CLI. Host `run-drot` / `ensure-drot-dist` no longer switch on `-Cli` / a separate `drot_tui` binary.
- Host `[profile.release]` favors daemon speed and practical multi-RID links (`opt-level = 3`, thin LTO, `panic = "abort"`, strip). DROT keeps fat-LTO `--release` for CI pack and thin-LTO `--profile dist` for local `target/dist/drot`.
- Replaced the binding pilot with product `dhara-sd`; `Dhara.Storage` NuGet uses the daemon (gRPC + handle transfer) instead of in-process FFI. Rust `dharastorage-ffi` remains for evidence benchmarks only.
- Added `Dhara.Storage.Extensions.Hosting` for Generic Host lifetime of the sidecar.
- Promoted BenchPilot to `benchmark/Dhara.Storage.Benchmarks` (FFI vs `dhara-sd` rung 1).
- DROT workspace crates live under `tooling/drot/crates/`; embed path is `core/dhara_storage/resources/filedefs.dat`.
- Relocated monorepo packages out of `src/`: `core/`, `interop/`, `bindings/csharp/`, and `benchmark/`. Tooling stays under `tooling/`.
- Native packaging / CI stages `dhara-sd` sidecars (not the FFI cdylib); managed tests and NuGet verify smoke run on the configured host/AOT RIDs.

### Fixed
- Honored `overwrite` on `StorageFile.CopyAsync` / `MoveAsync` when progress is null (always use the operation ABI).
- Directory watching waits for gRPC response headers before `StartWatching` returns so notify is attached before callers mutate the tree.
- File streams over duplicated handles use synchronous `FileStream` (handles are opened without `FILE_FLAG_OVERLAPPED`).

### Technical
- Bumped workspace / NuGet product line to **0.9.23**.
- Documented daemon transport, Windows code-signing (winresource vs Authenticode), binding benchmark ladder, and DROT one-binary / profile roles under `docs/` and AGENTS.
- `dhara-sd` embeds VERSIONINFO via `winresource`; named-pipe accept loop keeps a spare listener.

---

## v0.9.6 - 2026-07-26

### Added
- Added `dhara_storage_core` as the framework crate for DSFD schema, model, and FlatBuffers encode/decode (no embedded `filedefs.dat`).
- Added `tooling/drot` submodule ([dhara_repo_orchestration](https://github.com/D-Naveenz/dhara_repo_orchestration)) as the operator CLI/TUI, with CI downloading pinned DROT artifacts via `DROT_ARTIFACTS_TOKEN`.
- Added human-facing documentation contract: root vs project READMEs, `AGENTS.md` routing, and `docs/` for implementer reference.
- Added Linux `xvfb` support in CI so shell-icon coverage can run on headless runners.
- Added ratatui-based operator TUI (three-panel layout, interactive task tree, mouse/keyboard, modals) before the DROT extraction—progress milestones, audit logging, and daily session log append.
- Added Linux GUI / pkg-config dependency installation for GTK-based shell icons in CI.

### Changed
- Replaced in-tree `dhara_tool` with the DROT submodule; storage CI/CD and local verify scripts call `target/dist/drot`.
- Moved bundled `filedefs.dat` into `core/dhara_storage/resources/` (runtime `include_bytes!`); DROT sync targets that path.
- Nestled DSFD types under `dhara_storage_core::definitions` while keeping crate-root re-exports stable for callers.
- Clarified product split: core = framework/abstractions; `dhara_storage` = storage runtime (handles, analysis, watch, metadata).
- Moved primary .NET tests and NuGet verify onto Linux runners; Windows retained for MSVC native staging.
- Improved TrID `.7z` extraction (sevenz-rust with tar fallback) and entry-level extract progress reporting in the operator tooling lineage.

### Fixed
- Fixed headless Linux .NET shell-icon failures by running tests under Xvfb and skipping when the OS returns no icon (documented GTK limitation).
- Fixed macOS watcher path normalization for `/var` vs `/private/var`.
- Fixed MSVC relaunch quoting on Windows CI and Linux clippy unused-import gates for Windows-only code.
- Fixed merge-native PowerShell input handling in publish readiness.

### Removed
- Removed `dhara_storage_dal` (superseded by `dhara_storage_core`).
- Removed in-tree `dhara_tool` / iced GUI crates; operator UI lives in the DROT submodule.
- Removed obsolete MindVault / local tool layout assumptions from agent and project docs.

### Technical
- Bumped workspace / NuGet product line to **0.9.6**.
- Restructured DROT as kernel + `drot_dhara_storage` plugin + host crates (submodule); interim codec still uses published `dhara_storage_dal` from crates.io until core is the published codec dependency.
- Streamlined GitHub Actions pipeline (download-drot composite, Linux-first verify, action upgrades).
- Aligned inline rustdoc/XML docs and project-surface READMEs with the core vs runtime story.

---

## v0.9.0 - 2026-07-02

### Added
- Added dhara_tool CI build workflow with cached tool usage per OS/architecture.
- Added new dhara_tool subcommands for quality gates, native merge, and packaging replacing legacy shell scripts.
- Added MSVC environment support for native staging and release commands on Windows.
- Added dedicated test job for dhara_tool in CI running cargo test on Linux.
- Added VS Code tasks and launch configurations for production-shaped dhara_tool binary.
- Added `-r` / `--repository` flag for explicit repository root or config file specification.
- Added GUI repository picker overlay for interactive repository selection.
- Added Linux GUI dependencies setup composite GitHub Action for CI workflows.

### Changed
- Migrated primary CI jobs from Windows to Linux runners, retaining Windows runner for MSVC native DLL builds.
- Moved operator logs, artifacts, and outputs under executable directory `{tool_root}` (e.g., `target/dist/`).
- Activated configuration drift on startup, removing config sync command; added `--yes` flag for non-interactive drift application.
- Replaced legacy shell scripts with dhara_tool subcommands and updated CI/CD flow accordingly.
- Refactored GUI into primitives, promoted widgets, and screens with iced-based shell layout replacing monolithic panels.
- Synchronized tool versioning to workspace package version with shared workspace inheritance.
- Consolidated CI workflows and improved pipeline efficiency by running tests once and compiling per OS in separate jobs.
- Refined NuGet csproj synchronization with semantic metadata comparison and improved error handling.
- Updated documentation and config to reflect independent tool versioning and new CI/CD pipeline.

### Fixed
- Fixed Linux csproj path parsing to handle backslashes on non-Windows runners.
- Fixed LFS checkout requirement for embedded `filedefs.dat` during cargo test.
- Fixed stage-native cargo package name to use `dharastorage-ffi` for native assets.
- Fixed MSVC relaunch quoting issues on Windows CI.
- Fixed CI cache key to use source hash instead of version for dhara_tool.

### Removed
- Removed deprecated verify ci, verify docs, release publish, and native merge commands from dhara_tool CLI.
- Removed unused process module and merge_native_stages function.

### Technical
- Introduced source hash-based caching for dhara_tool binaries in CI.
- Updated GitHub Actions workflows for Linux-based smoke and AOT runtime tests.
- Added composite GitHub Action for Linux GUI and pkg-config dependency installation.
- Improved CI pipeline triggers and cache warming strategy.
- Enhanced dhara_tool cargo release and build scripts for better cache management.
- Updated VS Code launch configurations for dev and production-shaped builds.

---

## v0.8.0 - 2026-06-30

### Added
- Added cross-platform native asset staging and packaging support for Windows, Linux, and macOS in CI workflows.
- Added OS shell icon RGBA pixel support and cross-platform shell icon abstractions via `file_icon_provider` crate.
- Added detailed native packaging documentation covering multi-platform staging, merging, and packing.
- Added unified GitHub Actions pipeline consolidating CI and release workflows.
- Added structured logging for command execution with detailed start and end logs.
- Added new logging policy document outlining log level semantics and Dhara operator log requirements.
- Added MindVault integration for workspace memory with `mindvault.toml` storing workspace identity.
- Added detailed README files for core crates and operator CLI with architecture and usage examples.
- Added FlatBuffers encoding and decoding support
