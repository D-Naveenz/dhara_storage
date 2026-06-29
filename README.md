# Dhara Storage

[![dhara_storage on crates.io](https://img.shields.io/crates/v/dhara_storage?label=dhara_storage)](https://crates.io/crates/dhara_storage)
[![dhara_storage_dal on crates.io](https://img.shields.io/crates/v/dhara_storage_dal?label=dhara_storage_dal)](https://crates.io/crates/dhara_storage_dal)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE.txt)

Dhara Storage is a Rust-first storage runtime with a Windows-first delivery story.
It combines definition-driven file analysis, path-based file and directory operations,
debounced watching, a FlatBuffers-backed definitions data layer, and a managed .NET wrapper over
the native core.

Rust crates and the `Dhara.Storage` NuGet package are versioned together from
shared release metadata. The current publish target is `0.7.1`.

## Workspace

| Project                            | Purpose                                                                                               |
| ---------------------------------- | ----------------------------------------------------------------------------------------------------- |
| `src/core/dhara_storage_dal` | FlatBuffers data access layer; runtime defs at `src/core/dhara_storage_dal/resources/filedefs.dat` |
| `src/core/dhara_storage`     | Rust-native runtime for analysis, metadata, operations, navigation, and watching                      |
| `src/dharastorage`     | Thin C ABI over `dhara_storage` for managed and native hosts                                          |
| `src/bindings/Dhara.Storage`   | `net10.0` wrapper over `dharastorage`                                                                |
| `tooling/dhara_tool`           | Operator CLI for verification, packaging, release, and defs workflows                                 |

## Highlights

- Rust-native public API in `dhara_storage`, not a class-for-class port of the legacy C# model
- Bundled `filedefs.dat` runtime package for content-based file analysis
- File and directory operations that keep the simple path fast and opt into progress only when needed
- Debounced directory watching for stable change notifications
- Structured logging with `tracing` in Rust, native log forwarding through `dharastorage`, and host integration through `Microsoft.Extensions.Logging`
- Multi-runtime NuGet packaging for Windows `win-x64` and `win-arm64`

## Quick Start

Rust runtime:

```toml
[dependencies]
dhara_storage = "0.7.1"
```

```rust
use dhara_storage::{FileStorage, analyze_path};

let report = analyze_path("sample.pdf")?;
let bytes = FileStorage::from_existing("sample.pdf")?.read()?;
# Ok::<(), dhara_storage::StorageError>(())
```

.NET wrapper:

```powershell
dotnet add package Dhara.Storage --version 0.7.1
```

```csharp
using Microsoft.Extensions.Logging;
using Dhara.Storage;

using var loggerFactory = LoggerFactory.Create(builder => builder.AddConsole());
DharaStorage.UseLoggerFactory(loggerFactory);

var file = DharaStorage.File(@"C:\data\sample.pdf");
var analysis = file.Analyze();
var bytes = await file.ReadBytesAsync();
```

Tooling:

```powershell
./tooling/scripts/verify-local.ps1
cargo run -p dhara_tool -- verify package
cargo run -p dhara_tool -- release run --dry-run
```

## Support Matrix

| Surface                       | Status                                                                                      |
| ----------------------------- | ------------------------------------------------------------------------------------------- |
| `dhara_storage_dal`           | Shared FlatBuffers definitions DAL                                                          |
| `dhara_storage`               | Cross-platform core with Windows-first shell metadata; OS shell icons via `file_icon_provider` |
| `dharastorage`                | Cross-platform native ABI (`dll` / `so` / `dylib`)                                            |
| `Dhara.Storage` NuGet package | `win-x64`, `win-arm64`, `linux-x64`, `linux-arm64`, `osx-arm64`                             |

The NuGet package rejects unsupported 32-bit RIDs and platforms. Shell icons are returned as raw RGBA bytes (`ShellIcon`) when `includeIcon: true`; this is not PNG.

### Local Windows native builds (x64 + ARM64)

```powershell
rustup target add aarch64-pc-windows-msvc
# Visual Studio: Desktop development with C++ + MSVC x64/ARM64 build tools
$vcvars = "<VS>\VC\Auxiliary\Build\vcvarsall.bat"
cmd /c "call `"$vcvars`" x64_arm64 && cargo build -p dharastorage --release --target x86_64-pc-windows-msvc"
cmd /c "call `"$vcvars`" x64_arm64 && cargo build -p dharastorage --release --target aarch64-pc-windows-msvc"
```

Full five-RID package verification runs in CI after merging per-OS native stage artifacts. On Windows alone, `cargo run -p dhara_tool -- package stage-native` stages only `win-x64` and `win-arm64`.

## Logging

- Rust crates emit structured `tracing` events for analysis, metadata loading, operations, watching, package verification, and release flows.
- `dharastorage` exposes a native logger registration API that forwards JSON log records across the ABI.
- `Dhara.Storage` forwards both managed wrapper logs and native runtime logs into a host `ILoggerFactory`.
- `dhara_tool` now emits richer command, configuration, transfer, and verification logs for release diagnostics.

## Release Flow

- Shared release metadata lives in [dhara.config.toml](./dhara.config.toml).
- Local secrets belong in [.env.local](./.env.example), created from the example file.
- [tooling/dhara_tool](./tooling/dhara_tool/README.md) is the supported operator surface for config sync, verification, packaging, and publish flows.
- `cargo run -p dhara_tool -- release run --dry-run` validates the Cargo-first release flow without publishing.
- `cargo run -p dhara_tool -- release run` publishes `dhara_storage_dal` and `dhara_storage` first, then publishes the `Dhara.Storage` NuGet package.
- `cargo run -p dhara_tool -- release run --skip-cargo` publishes only the NuGet package when the Rust crates for the current version already exist.
- NuGet verification checks that all configured `runtimes/{rid}/native/*` entries are present in the package.

## Docs

- [dhara_storage_dal README](./src/core/dhara_storage_dal/README.md)
- [dhara_storage README](./src/core/dhara_storage/README.md)
- [dharastorage README](./src/dharastorage/README.md)
- [Dhara.Storage README](./src/bindings/Dhara.Storage/README.md)
