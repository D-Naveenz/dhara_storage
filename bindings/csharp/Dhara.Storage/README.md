# Dhara.Storage

[![NuGet](https://img.shields.io/nuget/v/Dhara.Storage)](https://www.nuget.org/packages/Dhara.Storage)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://github.com/D-Naveenz/dhara_storage/blob/main/LICENSE.txt)

.NET API for **file and directory handles**, content-based analysis, I/O with progress and cancellation, and directory watching—usable from ordinary libraries and desktop apps on Windows, Linux, and macOS.

The package ships **native** libraries for supported runtimes so storage and analysis work at native speed inside your .NET process. You work in C#; you do not need to write Rust.

## Why use it

- `DharaStorage.File` / `DharaStorage.Directory` for path-based work
- Content-based MIME and type candidates (`AnalyzePath` / analysis on handles)
- Sync and async read, write, copy, move, rename, delete
- Directory watching with typed change events
- One NuGet with native assets for supported RIDs

## Prerequisites

- .NET SDK **10.0.x**
- A supported runtime identifier at execution time (see below)

## Install

```powershell
dotnet add package Dhara.Storage --version 0.9.6
```

| OS | RIDs |
|----|------|
| Windows | `win-x64`, `win-arm64` |
| Linux | `linux-x64`, `linux-arm64` |
| macOS | `osx-arm64` |

## Usage

### 1. Optional logging

```csharp
using Microsoft.Extensions.Logging;
using Dhara.Storage;

using var loggerFactory = LoggerFactory.Create(builder => builder.AddConsole());
DharaStorage.UseLoggerFactory(loggerFactory);
```

### 2. Open a file, analyze, and read

```csharp
using Dhara.Storage;

var file = DharaStorage.File(@"C:\data\sample.pdf");
var info = file.RefreshInformation(includeAnalysis: true);
var bytes = await file.ReadBytesAsync();
```

### 3. Watch a directory

```csharp
using Dhara.Storage;

var directory = DharaStorage.Directory(@"C:\data");
directory.StartWatching();
directory.Changed += (_, change) => Console.WriteLine(change.Path);
```

Optional shell icons return raw RGBA pixels when `includeIcon: true`. `ShellDetails` is Windows-only today.

## Related

- Product overview: [Dhara Storage][root]
- Native ABI (advanced): [dharastorage][ffi]

## License

Apache-2.0.

[root]: https://github.com/D-Naveenz/dhara_storage
[ffi]: https://github.com/D-Naveenz/dhara_storage/blob/main/interop/dharastorage-ffi/README.md
