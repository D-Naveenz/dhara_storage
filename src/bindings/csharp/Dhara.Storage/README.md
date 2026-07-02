# Dhara.Storage

[![NuGet](https://img.shields.io/nuget/v/Dhara.Storage)](https://www.nuget.org/packages/Dhara.Storage)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://github.com/D-Naveenz/dhara_storage/blob/main/LICENSE.txt)

`Dhara.Storage` is the `net10.0` managed wrapper over the native [dharastorage][repo-dharastorage] runtime.
It exposes a path-based API for file analysis, metadata, I/O, directory operations, and watching without pushing .NET object shapes into the Rust core.

## ✨ Key Features

- **Path-based API** — `DharaStorage.File` and `DharaStorage.Directory` handles
- **Sync and async I/O** — reads, writes, copy, move, rename, delete
- **Point-in-time analysis** — `DharaStorage.AnalyzePath` without creating a handle
- **Directory watching** — explicit start/stop with typed change events
- **Logging integration** — managed wrapper + native `tracing` via `ILoggerFactory`
- **Multi-RID native assets** — one NuGet carries all supported runtimes

## 📦 Tech Stack & Architecture

| Piece | Role |
|-------|------|
| `Dhara.Storage` | Managed orchestration and public API |
| `dharastorage` | Native `cdylib` / `.so` / `.dylib` per RID |
| `dhara_storage` | Rust runtime behind the ABI |

```
Dhara.Storage/
├── DharaStorage.cs           # factory entry points
├── StorageFile.cs            # file handle API
├── StorageDirectory.cs       # directory handle + watching
├── Native/                   # P/Invoke and typed ABI marshalling
└── runtimes/{rid}/native/    # staged native libraries (packaged)
```

Typed ABI contract: [reference doc][typed-abi]. Workspace overview: [repo root][repo-root].

## 🚀 Getting Started & Installation

**Prerequisites**

- .NET SDK **10.0.x**
- A supported runtime identifier at execution time (see below)

```powershell
dotnet add package Dhara.Storage --version 0.9.0
```

**Supported RIDs**

| OS | RIDs |
|----|------|
| Windows | `win-x64`, `win-arm64` |
| Linux | `linux-x64`, `linux-arm64` |
| macOS | `osx-arm64` |

32-bit RIDs and unsupported platforms are rejected at build (`.targets`) and runtime (`PlatformNotSupportedException`).

## 🔧 Configuration & Environment Variables

No package-specific environment variables.

Wire logging before calling into storage:

```csharp
using var loggerFactory = LoggerFactory.Create(builder => builder.AddConsole());
DharaStorage.UseLoggerFactory(loggerFactory);
```

## 🛠️ Usage Examples

```csharp
using Microsoft.Extensions.Logging;
using Dhara.Storage;

using var loggerFactory = LoggerFactory.Create(builder => builder.AddConsole());
DharaStorage.UseLoggerFactory(loggerFactory);

var file = DharaStorage.File(@"C:\data\sample.pdf");
var info = file.RefreshInformation(includeAnalysis: true);
var bytes = await file.ReadBytesAsync();

var directory = DharaStorage.Directory(@"C:\data");
directory.StartWatching();
directory.Changed += (_, change) => Console.WriteLine(change.Path);
```

**Shell icons (RGBA, not PNG)**

When `includeIcon: true`, `ShellIcon` carries row-major RGBA pixels (`Width`, `Height`, `RgbaPixels`).
Upload directly to Skia, ImageSharp, or similar. Encode to PNG in managed code if you need a file format.

`ShellDetails` (display name / type) is **Windows-only** today.

**Troubleshooting**

- `PlatformNotSupportedException` → check RID matches a packaged `runtimes/{rid}/native` asset.
- Missing native DLL on local `dotnet build` → build `dharastorage` first; workspace dev copies the local native lib.
- Do not rely on bare `dotnet pack` for release-shaped packages — use workspace [dhara_tool][repo-tool] staging.

## ✅ Testing & Quality Assurance

From the workspace root (Windows):

```powershell
dotnet test src/bindings/csharp/Dhara.Storage.Tests/Dhara.Storage.Tests.csproj
./tooling/scripts/verify-local.ps1
```

Full five-RID package verification runs in CI after per-OS native merge — see [CI/CD reference][ci-cd].

## 🤝 Contributing & License

Part of the [Dhara Storage workspace][repo-root]. NuGet package licensed under Apache-2.0.

[repo-root]: https://github.com/D-Naveenz/dhara_storage
[repo-dharastorage]: https://github.com/D-Naveenz/dhara_storage/tree/main/src/bindings/dharastorage-ffi
[repo-tool]: https://github.com/D-Naveenz/dhara_storage/tree/main/tooling/dhara_tool
[typed-abi]: https://github.com/D-Naveenz/dhara_storage/blob/main/docs/typed-c-compatible-abi.md
[ci-cd]: https://github.com/D-Naveenz/dhara_storage/blob/main/docs/ci-cd-pipelines.md
