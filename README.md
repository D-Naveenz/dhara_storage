# Dhara Storage

[![dhara_storage on crates.io](https://img.shields.io/crates/v/dhara_storage?label=dhara_storage)](https://crates.io/crates/dhara_storage)
[![dhara_storage_dal on crates.io](https://img.shields.io/crates/v/dhara_storage_dal?label=dhara_storage_dal)](https://crates.io/crates/dhara_storage_dal)
[![Dhara.Storage on NuGet](https://img.shields.io/nuget/v/Dhara.Storage?label=Dhara.Storage)](https://www.nuget.org/packages/Dhara.Storage)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE.txt)

Dhara Storage is a cross-platform local storage runtime for applications. You work with file and directory handles—read, write, copy, move, watch, and metadata—and when you need to know what a file really is, you get **content-based type intelligence**, not a guess from the extension.

Ordinary file APIs stop at paths and bytes. Platform “content type” fields often stop at the name. Dhara combines a storage-handle model with signature-based analysis in one runtime, so business apps and libraries can classify and manage files without a separate MIME stack—and without being locked to one OS app model.

The core is written in Rust for memory safety, thread safety, and fearless concurrency under heavy I/O and analysis workloads.

## Why use it

- Know the real type from file bytes (bundled definitions), not just the extension
- File and directory handles with transfers that support progress and cancellation
- Built-in directory watching and shell-aware metadata where the OS provides it
- Use it from libraries and desktop apps on Windows, Linux, and macOS—same idea everywhere
- Rust core when concurrency and system-level correctness matter

## Quick start

**Prerequisites:** Rust stable (for the crate), and/or .NET SDK 10.0.x (for the NuGet package).

### Rust

```toml
[dependencies]
dhara_storage = "0.9.0"
```

```rust
use dhara_storage::{FileStorage, analyze_path};

let report = analyze_path("sample.pdf")?;
let bytes = FileStorage::from_existing("sample.pdf")?.read()?;
# Ok::<(), dhara_storage::StorageError>(())
```

More: [dhara_storage crate README][readme-dhara-storage].

### .NET

```powershell
dotnet add package Dhara.Storage --version 0.9.0
```

```csharp
using Dhara.Storage;

var file = DharaStorage.File(@"C:\data\sample.pdf");
var info = file.RefreshInformation(includeAnalysis: true);
var bytes = await file.ReadBytesAsync();
```

More: [Dhara.Storage package README][readme-nuget].

## What’s in this repository

| Package | README | Surface |
|---------|--------|---------|
| `dhara_storage` | [crate][readme-dhara-storage] | crates.io — Rust runtime |
| `dhara_storage_dal` | [crate][readme-dal] | crates.io — file definition package |
| `dharastorage` | [FFI][readme-dharastorage] | Native C ABI (NuGet asset) |
| `Dhara.Storage` | [NuGet][readme-nuget] | .NET bindings |
| Operator CLI | [DROT][readme-tool] | Submodule — build/release tooling |

Technical reference (ABI, CI, definition format): [docs index][docs-index].

Contributor / agent context (architecture, local verify, CI): [AGENTS.md][agents].

## Contributing & license

Open a pull request against `main`. Keep package READMEs accurate when public behavior or publish surfaces change.

Licensed under [Apache-2.0][license].

[readme-dhara-storage]: src/core/dhara_storage/README.md
[readme-dal]: src/core/dhara_storage_dal/README.md
[readme-dharastorage]: src/bindings/dharastorage-ffi/README.md
[readme-nuget]: src/bindings/csharp/Dhara.Storage/README.md
[readme-tool]: tooling/drot/README.md
[docs-index]: docs/README.md
[agents]: AGENTS.md
[license]: LICENSE.txt
