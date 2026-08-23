# dhara_storage

[![crates.io](https://img.shields.io/crates/v/dhara_storage)](https://crates.io/crates/dhara_storage)
[![docs.rs](https://img.shields.io/docsrs/dhara_storage)](https://docs.rs/dhara_storage)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://github.com/D-Naveenz/dhara_storage/blob/main/LICENSE.txt)

Rust runtime for **content-based file analysis**, file and directory handles, transfers with progress and cancellation, directory watching, and shell metadata.

Use this crate when you want those capabilities from Rust—with memory safety, thread safety, and fearless concurrency under heavy I/O and analysis.

## Why this crate

- Signature-based typing via bundled `filedefs.dat` (not extension-only guessing)
- `FileStorage` / `DirectoryStorage` handles for navigation and I/O
- Sync-first copy / move / delete; copy/move start a `StorageProcess` (await or wait) with optional progress and cancellation
- Debounced directory change events
- Shell icons (RGBA) on supported desktops
- Optional Tokio wrappers (`async-tokio`)

## Prerequisites

- Rust **stable** toolchain (`cargo`)

## Install

```toml
[dependencies]
dhara_storage = "0.9.24"
```

Optional async:

```toml
dhara_storage = { version = "0.9.24", features = ["async-tokio"] }
```

## Usage

### 1. Analyze a path

```rust
use dhara_storage::analyze_path;

let report = analyze_path("sample.png")?;
# Ok::<(), dhara_storage::StorageError>(())
```

### 2. Open a file handle and read

```rust
use dhara_storage::FileStorage;

let bytes = FileStorage::from_existing("sample.png")?.read()?;
# Ok::<(), dhara_storage::StorageError>(())
```

### 3. List a directory

```rust
use dhara_storage::DirectoryStorage;

let files = DirectoryStorage::from_existing(".")?.files()?;
# Ok::<(), dhara_storage::StorageError>(())
```

### 4. Copy with progress

```rust
use std::sync::Arc;
use dhara_storage::{FileStorage, StorageProcessEvent, TransferOptions};

let progress = Arc::new(|event: StorageProcessEvent| {
    if let StorageProcessEvent::Bytes { bytes_transferred } = event {
        println!("{bytes_transferred} bytes");
    }
});

// Sync: starts a process and waits inside (returns ()).
FileStorage::from_existing("input.bin")?.copy_to_with_options(
    "output.bin",
    TransferOptions {
        overwrite: true,
        progress: Some(progress),
        ..TransferOptions::default()
    },
)?;

// Or start and await the process (destination on ProcessOutcome).
// let outcome = FileStorage::from_existing("input.bin")?
//     .start_copy_to_with_options("output.bin", TransferOptions { overwrite: true, ..Default::default() })
//     .await?;
# Ok::<(), dhara_storage::StorageError>(())
```

## Platform notes

| Capability | Windows | Linux | macOS |
|------------|---------|-------|-------|
| Analysis, I/O, watching | yes | yes | yes |
| `ShellIcon` (RGBA) | yes | yes* | yes |

\*Linux GTK icons may require the main thread. `ShellIcon` returns raw RGBA pixels—encode to PNG in your app if needed.

## Related

- Framework crate (DSFD, progress/cancel, portable types): [dhara_storage_core][core]
- Product overview: [Dhara Storage][root]
- API docs: [docs.rs/dhara_storage][docs-rs]

Extension crates may depend on this runtime and compose `FileStorage` / `DirectoryStorage` (for example a future archives crate). Apps that need a closed mixed collection of handle kinds should use a consumer-owned enum.

## License

Apache-2.0.

[core]: https://github.com/D-Naveenz/dhara_storage/blob/main/core/dhara_storage_core/README.md
[root]: https://github.com/D-Naveenz/dhara_storage
[docs-rs]: https://docs.rs/dhara_storage
