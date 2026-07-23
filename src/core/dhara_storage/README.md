# dhara_storage

[![crates.io](https://img.shields.io/crates/v/dhara_storage)](https://crates.io/crates/dhara_storage)
[![docs.rs](https://img.shields.io/docsrs/dhara_storage)](https://docs.rs/dhara_storage)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://github.com/D-Naveenz/dhara_storage/blob/main/LICENSE.txt)

`dhara_storage` is the Rust runtime for Dhara Storage: **content-based file analysis**, file and directory handles, transfers with progress and cancellation, directory watching, and shell metadata.

Rust is the implementation language so analysis and I/O can run with memory safety, thread safety, and fearless concurrency—without pushing host-language object models into the core.

## Why this crate

- Signature-based typing via bundled `filedefs.dat` (not extension-only guessing)
- `FileStorage` / `DirectoryStorage` handles for navigation and I/O
- Sync-first copy/move/delete with optional progress and cancellation
- Debounced directory change events
- Shell icons (RGBA) and Windows shell details where supported
- Optional Tokio wrappers (`async-tokio` feature)

## Install

**Prerequisites:** Rust stable.

```toml
[dependencies]
dhara_storage = "0.9.0"
```

Optional async:

```toml
dhara_storage = { version = "0.9.0", features = ["async-tokio"] }
```

## Usage

```rust
use dhara_storage::{FileStorage, analyze_path};

let report = analyze_path("sample.png")?;
let bytes = FileStorage::from_existing("sample.png")?.read()?;
# Ok::<(), dhara_storage::StorageError>(())
```

```rust
use dhara_storage::DirectoryStorage;

let directory = DirectoryStorage::from_existing(".")?;
let files = directory.files()?;
# Ok::<(), dhara_storage::StorageError>(())
```

Install a `tracing` subscriber in your app if you want structured logs.

## Platform notes

| Capability | Windows | Linux | macOS |
|------------|---------|-------|-------|
| Analysis, I/O, watching | yes | yes | yes |
| `ShellIcon` (RGBA) | yes | yes* | yes |
| `ShellDetails` | yes | no | no |

\*Linux GTK icons may require the main thread. `ShellIcon` returns raw RGBA pixels—encode to PNG in your app if needed.

## Related packages

- Definitions package: [dhara_storage_dal][repo-dal]
- C ABI / .NET: [dharastorage][repo-dharastorage], [Dhara.Storage][repo-nuget]
- Workspace overview: [repo root][repo-root]
- DSFD format: [filedefs reference][filedefs-dat]

## License

Apache-2.0. Part of the [Dhara Storage workspace][repo-root].

[repo-root]: https://github.com/D-Naveenz/dhara_storage
[repo-dal]: https://github.com/D-Naveenz/dhara_storage/tree/main/src/core/dhara_storage_dal
[repo-dharastorage]: https://github.com/D-Naveenz/dhara_storage/tree/main/src/bindings/dharastorage-ffi
[repo-nuget]: https://github.com/D-Naveenz/dhara_storage/tree/main/src/bindings/csharp/Dhara.Storage
[filedefs-dat]: https://github.com/D-Naveenz/dhara_storage/blob/main/docs/filedefs-dat.md
