# dharastorage

[![License: Apache-2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://github.com/D-Naveenz/dhara_storage/blob/main/LICENSE.txt)

`dharastorage` is the stable **C ABI** over the [dhara_storage][repo-dhara-storage] Rust runtime. It lets non-Rust hosts (notably [.NET Dhara.Storage][repo-nuget]) call analysis, I/O, watching, and related operations without linking Rust types directly.

Filesystem and analysis behavior stay in the core crate; this layer marshals results across the boundary.

## Who should use this

- **.NET apps** — prefer [Dhara.Storage][repo-nuget]; do not call the ABI by hand unless you must.
- **Other FFI hosts** — link the native library and follow the typed ABI contract.

This crate is built as a workspace member and staged into NuGet — not published as a standalone crates.io product.

## Build

```powershell
cargo build -p dharastorage-ffi --release
```

## Capabilities (via ABI)

- Immediate queries: analysis, metadata, listings, reads, writes, path mutations
- Background ops: copy, move, delete, read, write with progress and cancellation
- Directory watches with debounced typed events
- Streaming write sessions for managed hosts
- Logger bridge from `tracing` to a host callback

Ownership and layout rules: [typed C-compatible ABI][typed-abi].

## License

Apache-2.0. Part of the [Dhara Storage workspace][repo-root].

[repo-root]: https://github.com/D-Naveenz/dhara_storage
[repo-dhara-storage]: https://github.com/D-Naveenz/dhara_storage/tree/main/src/core/dhara_storage
[repo-nuget]: https://github.com/D-Naveenz/dhara_storage/tree/main/src/bindings/csharp/Dhara.Storage
[typed-abi]: https://github.com/D-Naveenz/dhara_storage/blob/main/docs/typed-c-compatible-abi.md
