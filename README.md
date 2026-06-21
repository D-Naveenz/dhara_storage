# Dhara Storage

[![dhara_storage on crates.io](https://img.shields.io/crates/v/dhara_storage?label=dhara_storage)](https://crates.io/crates/dhara_storage)
[![dhara_dhbin on crates.io](https://img.shields.io/crates/v/dhara_dhbin?label=dhara_dhbin)](https://crates.io/crates/dhara_dhbin)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE.txt)

Dhara Storage is a Rust-first storage runtime with a Windows-first delivery story.
It combines definition-driven file analysis, path-based file and directory operations,
debounced watching, a reusable `DHBIN` package format, and a managed .NET wrapper over
the native core.

Rust crates are already available on crates.io and are being advanced from `0.4.0`
to `0.4.4`. The first `Dhara.Storage` NuGet package is prepared for `0.4.4`.

## Workspace

| Project                            | Purpose                                                                                               |
| ---------------------------------- | ----------------------------------------------------------------------------------------------------- |
| `src/static/dhara_dhbin`       | Shared `DHBIN` v2 container crate for MessagePack payloads, optional metadata, and integrity sections |
| `src/static/dhara_storage`     | Rust-native runtime for analysis, metadata, operations, navigation, and watching                      |
| `src/dynamic/dharastorage`     | Thin C ABI over `dhara_storage` for managed and native hosts                                          |
| `src/bindings/Dhara.Storage`   | `net10.0` wrapper over `dharastorage`                                                                |
| `tooling/dhara_tool`           | Operator CLI for verification, packaging, release, and defs workflows                                 |
| `tooling/dhara_storage_ops`    | Repository-specific capability pack used by `dhara_tool`                                             |

## Highlights

- Rust-native public API in `dhara_storage`, not a class-for-class port of the legacy C# model
- Bundled `filedefs.dhbin` runtime package for content-based file analysis
- File and directory operations that keep the simple path fast and opt into progress only when needed
- Debounced directory watching for stable change notifications
- Structured logging with `tracing` in Rust, native log forwarding through `dharastorage`, and host integration through `Microsoft.Extensions.Logging`
- Multi-runtime NuGet packaging for Windows `win-x64` and `win-arm64`

## Quick Start

Rust runtime:

```toml
[dependencies]
dhara_storage = "0.4.4"
```

```rust
use dhara_storage::{FileStorage, analyze_path};

let report = analyze_path("sample.pdf")?;
let bytes = FileStorage::from_existing("sample.pdf")?.read()?;
# Ok::<(), dhara_storage::StorageError>(())
```

.NET wrapper:

```powershell
dotnet add package Dhara.Storage --version 0.4.4
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
cargo run -p dhara_tool -- verify ci
cargo run -p dhara_tool -- verify package
cargo run -p dhara_tool -- release run --dry-run
```

## Support Matrix

| Surface                       | Status                                                                          |
| ----------------------------- | ------------------------------------------------------------------------------- |
| `dhara_dhbin`                 | Portable Rust crate                                                             |
| `dhara_storage`               | Windows-first runtime; portable where the underlying functionality naturally is |
| `dharastorage`                 | Windows-first native ABI                                                        |
| `Dhara.Storage` NuGet package | Windows `win-x64` and `win-arm64` only                                          |

The NuGet package now fails clearly during package consumption for unsupported RIDs such as `win-x86`,
and the managed wrapper also throws a `PlatformNotSupportedException` when loaded outside Windows `x64` or `arm64`.

## Logging

- Rust crates emit structured `tracing` events for analysis, metadata loading, operations, watching, package verification, and release flows.
- `dharastorage` exposes a native logger registration API that forwards JSON log records across the ABI.
- `Dhara.Storage` forwards both managed wrapper logs and native runtime logs into a host `ILoggerFactory`.
- `dhara_tool` and `dhara_storage_ops` now emit richer command, configuration, transfer, and verification logs for release diagnostics.

## Release Flow

- Shared release metadata lives in [dhara.config.toml](./dhara.config.toml).
- Local secrets belong in [.env.local](./.env.example), created from the example file.
- [tooling/dhara_tool](./tooling/dhara_tool/README.md) is the supported operator surface for config sync, verification, packaging, and publish flows.
- `cargo run -p dhara_tool -- release run --dry-run` validates the Cargo-first release flow without publishing.
- `cargo run -p dhara_tool -- release run` publishes Cargo first, then publishes the `Dhara.Storage` NuGet package.
- `cargo run -p dhara_tool -- release run --skip-cargo` publishes only the NuGet package when the Rust crates for the current version already exist.
- NuGet verification checks that both `runtimes/win-x64/native/dharastorage.dll` and `runtimes/win-arm64/native/dharastorage.dll` are present in the package.

## Docs

- [dhara_dhbin README](./src/static/dhara_dhbin/README.md)
- [dhara_storage README](./src/static/dhara_storage/README.md)
- [dharastorage README](./src/dynamic/dharastorage/README.md)
- [Dhara.Storage README](./src/bindings/Dhara.Storage/README.md)
