# dharastorage

[![License: Apache-2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://github.com/D-Naveenz/dhara_storage/blob/main/LICENSE.txt)

Stable **C ABI** over the Dhara Storage Rust runtime. Use this when a non-Rust host must call analysis, I/O, watching, and related operations without linking Rust types.

Filesystem and analysis behavior live in the core; this crate marshals results across the boundary.

**.NET applications** should use [Dhara.Storage][nuget] instead of calling this ABI directly.

## Why this package

- Immediate queries: analysis, metadata, listings, reads, writes, path mutations
- Background ops with progress and cancellation
- Directory watches with debounced typed events
- Streaming write sessions for managed hosts

## Prerequisites

- Rust **stable** toolchain
- Familiarity with C FFI ownership (`*_free`, UTF-8 pointer/length strings)

## Build

From the Dhara Storage workspace root:

```powershell
cargo build -p dharastorage-ffi --release
```

This crate is an evidence / benchmark ABI (and a C interop option). The [Dhara.Storage][nuget] package ships the `dhara-sd` sidecar instead of this cdylib.

## Usage

### 1. Link the native library

Build the `cdylib` for your target, then load it from your host language.

### 2. Call typed exports

Representative entry points (see source for the full list):

- `dhara_analyze_path`
- `dhara_get_file_metadata` / `dhara_get_directory_metadata`
- `dhara_list_files` / `dhara_list_directories` / `dhara_list_entries`
- Watch helpers: `dhara_watch_try_recv_event`, `dhara_watch_recv_event`, …

### 3. Follow ownership rules

- Hot structured results use Rust-owned `#[repr(C)]` handles — copy what you need, then call the matching `*_free`
- Strings are UTF-8 pointer/length slices
- JSON is for errors and diagnostics—not hot query paths

Full contract: [typed C-compatible ABI][typed-abi].

## Related

- .NET package: [Dhara.Storage][nuget]
- Rust runtime: [dhara_storage][runtime]
- Product overview: [Dhara Storage][root]

## License

Apache-2.0.

[nuget]: https://github.com/D-Naveenz/dhara_storage/blob/main/bindings/csharp/Dhara.Storage/README.md
[runtime]: https://github.com/D-Naveenz/dhara_storage/blob/main/core/dhara_storage/README.md
[root]: https://github.com/D-Naveenz/dhara_storage
[typed-abi]: https://github.com/D-Naveenz/dhara_storage/blob/main/docs/typed-c-compatible-abi.md
