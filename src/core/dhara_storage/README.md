# dhara_storage

[![crates.io](https://img.shields.io/crates/v/dhara_storage)](https://crates.io/crates/dhara_storage)
[![docs.rs](https://img.shields.io/docsrs/dhara_storage)](https://docs.rs/dhara_storage)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://github.com/D-Naveenz/dhara_storage/blob/main/LICENSE.txt)

`dhara_storage` is the Rust-native runtime for Dhara Storage.
It provides definition-driven file analysis, path-based file and directory operations, debounced watching, and structured `tracing` diagnostics.
FFI and managed layers stay thin and delegate behavior here.

## ✨ Key Features

- **Content analysis** — matches paths against bundled `filedefs.dat` definitions
- **Storage handles** — `FileStorage` and `DirectoryStorage` for navigation and I/O
- **Transfer options** — sync-first copy/move with optional progress and cancellation
- **Directory watching** — debounced change events via `notify`
- **Shell metadata** — RGBA icons (`ShellIcon`) and Windows shell details where supported
- **Optional async** — Tokio-backed wrappers behind the `async-tokio` feature

## 📦 Tech Stack & Architecture

| Piece | Role |
|-------|------|
| `dhara_storage_dal` | Embedded FlatBuffers definition package |
| `file_icon_provider` | Cross-platform shell icon pixels |
| `notify` | Filesystem watcher backend |
| `tracing` | Structured runtime instrumentation |

```
dhara_storage/src/
├── analysis/          # definition-driven file typing
├── operations/        # copy, move, delete, read, write
├── storage/           # FileStorage, DirectoryStorage handles
├── watching/          # debounced directory events
└── metadata/          # shell icon and display metadata
```

Higher layers: [dharastorage][repo-dharastorage] (C ABI) and [Dhara.Storage][repo-nuget] (.NET).

## 🚀 Getting Started & Installation

**Prerequisites:** Rust stable.

```toml
[dependencies]
dhara_storage = "0.7.1"
```

Optional async wrappers:

```toml
dhara_storage = { version = "0.7.1", features = ["async-tokio"] }
```

## 🔧 Configuration & Environment Variables

No crate-specific environment variables. Install a `tracing` subscriber in your app before calling into the runtime if you want structured logs on stdout or in your aggregator.

Definition package updates are workspace concerns — see [dhara_storage_dal][repo-dal] and [DSFD reference][filedefs-dat].

## 🛠️ Usage Examples

**Analyze and read**

```rust
use dhara_storage::{FileStorage, analyze_path};

let report = analyze_path("sample.png")?;
let bytes = FileStorage::from_existing("sample.png")?.read()?;
# Ok::<(), dhara_storage::StorageError>(())
```

**Enumerate a directory**

```rust
use dhara_storage::DirectoryStorage;

let directory = DirectoryStorage::from_existing(".")?;
let files = directory.files()?;
# Ok::<(), dhara_storage::StorageError>(())
```

**Progress-aware copy**

```rust
use std::sync::Arc;
use dhara_storage::{FileStorage, StorageProgress, TransferOptions};

let progress = Arc::new(|update: StorageProgress| {
    println!("{} bytes", update.bytes_transferred);
});

FileStorage::from_existing("input.bin")?.copy_to_with_options(
    "output.bin",
    TransferOptions {
        overwrite: true,
        buffer_size: None,
        progress: Some(progress),
        cancellation_token: None,
    },
)?;
# Ok::<(), dhara_storage::StorageError>(())
```

**Platform notes**

| Capability | Windows | Linux | macOS |
|------------|---------|-------|-------|
| Analysis, I/O, watching | yes | yes | yes |
| `ShellIcon` (RGBA) | yes | yes* | yes |
| `ShellDetails` | yes | no | no |

\*Linux GTK icons may require the main thread.

`ShellIcon` returns raw RGBA pixels — encode to PNG in your app if needed.

**Troubleshooting**

- Analysis misses expected types → refresh embedded defs via workspace `dhara_tool defs sync-embedded`; see [filedefs reference][filedefs-dat].
- No log output → install a `tracing` subscriber before first crate call.

## ✅ Testing & Quality Assurance

From the workspace root:

```powershell
cargo test -p dhara_storage --all-features
cargo clippy -p dhara_storage --all-targets --all-features -- -D warnings
```

API docs: [docs.rs/dhara_storage][docs-rs].

## 🤝 Contributing & License

Part of the [Dhara Storage workspace][repo-root]. Licensed under Apache-2.0.

Deep references: [typed C ABI][typed-abi], [logging conventions][logging].

[repo-root]: https://github.com/D-Naveenz/dhara_storage
[repo-dal]: https://github.com/D-Naveenz/dhara_storage/tree/main/src/core/dhara_storage_dal
[repo-dharastorage]: https://github.com/D-Naveenz/dhara_storage/tree/main/src/dharastorage
[repo-nuget]: https://github.com/D-Naveenz/dhara_storage/tree/main/src/bindings/Dhara.Storage
[filedefs-dat]: https://github.com/D-Naveenz/dhara_storage/blob/main/docs/filedefs-dat.md
[typed-abi]: https://github.com/D-Naveenz/dhara_storage/blob/main/docs/typed-c-compatible-abi.md
[logging]: https://github.com/D-Naveenz/dhara_storage/blob/main/docs/logging.md
[docs-rs]: https://docs.rs/dhara_storage
