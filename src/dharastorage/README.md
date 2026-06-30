# dharastorage

[![License: Apache-2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://github.com/D-Naveenz/dhara_storage/blob/main/LICENSE.txt)

`dharastorage` is the native C ABI layer over [dhara_storage][repo-dhara-storage].
It exposes a stable, UTF-8-oriented FFI for [Dhara.Storage][repo-nuget] and other hosts that cannot link Rust types directly.
Filesystem behavior stays in the core crate; this crate marshals results across the boundary.

## ✨ Key Features

- **Immediate queries** — analysis, metadata, listings, reads, writes, path mutations
- **Background operations** — copy, move, delete, read, write with progress and cancellation
- **Directory watches** — debounced typed native events
- **Streaming writes** — chunked upload sessions for managed hosts
- **Typed hot-path ABI** — `#[repr(C)]` result handles with matching `*_free` functions
- **Logger bridge** — forwards `tracing` events as JSON to a host callback

## 📦 Tech Stack & Architecture

| Piece | Role |
|-------|------|
| `dhara_storage` | All filesystem and analysis behavior |
| `serde_json` | Cold-path errors, operation errors, log records |
| `cdylib` | Native library shipped inside NuGet `runtimes/` |

```
dharastorage/src/
├── ffi/           # exported C entry points
├── typed/         # #[repr(C)] result structs and owners
├── operations/    # background handle lifecycle
└── logging/       # dhara_register_logger bridge
```

ABI design rules and ownership patterns: [typed C-compatible ABI reference][typed-abi].

## 🚀 Getting Started & Installation

**Prerequisites:** Rust stable. This crate is built as a workspace member and staged into NuGet — not published standalone to crates.io.

Build from the workspace root:

```powershell
cargo build -p dharastorage --release
```

Multi-RID packaging uses [dhara_tool `package stage-native`][repo-tool] and [`native merge`][repo-tool] in CI — see [CI/CD reference][ci-cd].

## 🔧 Configuration & Environment Variables

No crate-specific environment variables.

Hosts register a logger with `dhara_register_logger` to receive UTF-8 JSON log records (level, target, message, timestamp, optional file/line, structured fields).

## 🛠️ Usage Examples

**Representative typed exports** (full list in source):

- `dhara_analyze_path`
- `dhara_get_file_info` / `dhara_get_directory_info`
- `dhara_list_files` / `dhara_list_directories` / `dhara_list_entries`
- `dhara_watch_try_recv_event` / `dhara_watch_recv_event` / `dhara_watch_recv_event_timeout`

**ABI policy (summary)**

- Hot structured results use Rust-owned `#[repr(C)]` handles — copy immediately, then call the matching `*_free`.
- Strings in results are UTF-8 pointer/length slices, not embedded host strings.
- JSON is reserved for errors, diagnostics, and logging — not hot query paths.

**.NET consumers** should use [Dhara.Storage][repo-nuget] rather than calling this ABI directly.

**Troubleshooting**

- Layout mismatches between Rust and C# → follow [typed ABI reference][typed-abi]; add layout tests when changing structs.
- Memory leaks → ensure every success path calls the matching `*_free` in a `finally` block.

## ✅ Testing & Quality Assurance

```powershell
cargo test -p dharastorage
cargo clippy -p dharastorage --all-targets -- -D warnings
```

Integration coverage also runs through [Dhara.Storage.Tests][repo-nuget] on Windows CI.

## 🤝 Contributing & License

Part of the [Dhara Storage workspace][repo-root]. Licensed under Apache-2.0.

[repo-root]: https://github.com/D-Naveenz/dhara_storage
[repo-dhara-storage]: https://github.com/D-Naveenz/dhara_storage/tree/main/src/core/dhara_storage
[repo-nuget]: https://github.com/D-Naveenz/dhara_storage/tree/main/src/bindings/Dhara.Storage
[repo-tool]: https://github.com/D-Naveenz/dhara_storage/tree/main/tooling/dhara_tool
[typed-abi]: https://github.com/D-Naveenz/dhara_storage/blob/main/docs/typed-c-compatible-abi.md
[ci-cd]: https://github.com/D-Naveenz/dhara_storage/blob/main/docs/ci-cd-pipelines.md
