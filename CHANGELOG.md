# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased - 2026-07-09

### Added
- Added ratatui-based TUI replacing iced GUI with a three-panel layout and interactive task tree adapter module supporting keyboard and mouse input.
- Added multi-step operation progress bar and status tracking in TUI with detailed extract progress reporting (e.g., "Extracting archive (k/N)").
- Added CommandRun RAII guard for consistent audit logging of command lifecycle events with elapsed time.
- Added InteractiveDiagnosticLayer to route WARN/ERROR logs to TUI panel in interactive mode.
- Added mouse event handling and marquee text support for task tree UI.
- Added modal dialogs for repository setup and activation prompts with full keyboard and mouse support.
- Added daily log file appending with session end records for improved log aggregation.
- Added CLI launch logic to detect interactive terminals and launch TUI instead of GUI.
- Added Linux GUI and pkg-config dependency installation step in CI workflows.
- Added semantic version snapshot comparison in cargo_toml sync to avoid unnecessary rewrites.

### Changed
- Replaced iced-based GUI with ratatui-based TUI for dhara_tool operator interface.
- Simplified action panel title to constant "Actions" and standardized progress step labels without dynamic counts or "— done" suffix.
- Improved TrID .7z archive extraction using sevenz-rust with fallback to tar extraction.
- Refined TUI UI styling: bold warning style for tree view selection, removed cursor indicators, reduced vertical spacing, and simplified command bar and title bar styling.
- Refactored TUI app state to include focus management, tab views, and modular input handling using ratatui_interact.
- Refactored quality, package, verify, and release workflows to use planned multi-step progress lifecycle with nested plan detection.
- Changed daily log files to append all sessions instead of overwriting.
- Updated CLI to launch TUI on interactive terminals and fallback to terminal launch.
- Refined cargo_toml sync logic to preserve content when versions match.
- Updated CI workflows to run tests and verification on Linux runners instead of Windows.
- Removed background elapsed time reporter; elapsed time computed on worker thread snapshots.
- Updated documentation and progress logic to reflect new progress reporting and archive extraction fallback.

### Fixed
- Fixed progress flicker by limiting worker-thread snapshots and removing background reporter.
- Fixed status flicker by isolating OperationPlan per thread and using command milestone for panel title.
- Fixed cargo_toml sync to ignore formatting differences and avoid unnecessary file rewrites.
- Fixed MSVC relaunch quoting on Windows CI by using temporary batch files and stripping quotes.
- Fixed Linux clippy unused-import warnings by gating Windows-only imports.
- Fixed merge-native PowerShell input array handling in publish readiness job.
- Fixed macOS watcher path normalization to handle /var vs /private/var symlinks.
- Fixed TrID archive extraction progress reporting with entry-level ticks.
- Fixed CI workflows to install Linux GUI dependencies before building on cache miss.

### Deprecated
- Removed dynamic command milestone from action panel title to reduce flicker and confusion.

### Removed
- Removed iced-based GUI crate and assets.
- Removed background elapsed time reporter and redundant UI elements in TUI action panel.
- Removed unused fields from TUI FocusState and cleaned up theme constants.
- Removed redundant UI elements and spinner from TUI progress rendering.
- Removed unused background color definition from theme constants.

### Technical
- Refactored cargo_toml sync logic with semantic version snapshot comparison.
- Refactored TUI architecture for modularity and theme awareness.
- Updated CI workflows to unify Linux runners for tests and verification.
- Upgraded GitHub Actions versions and streamlined pipeline steps.
- Cleaned up unused imports in dhara_tool_cli package commands.
- Refined worker thread snapshot publishing to reduce UI flicker.

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