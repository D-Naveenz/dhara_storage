# Dhara.Storage

[![NuGet](https://img.shields.io/nuget/v/Dhara.Storage)](https://www.nuget.org/packages/Dhara.Storage)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://github.com/D-Naveenz/dhara_storage/blob/main/LICENSE.txt)

`Dhara.Storage` is the .NET API for Dhara Storage: **file and directory handles**, content-based analysis, I/O with progress and cancellation, and directory watching—usable from ordinary libraries and desktop apps on Windows, Linux, and macOS.

You get a storage-handle model plus signature-based type intelligence in one package, without being locked to a single OS app framework.

## Why use it

- `DharaStorage.File` / `DharaStorage.Directory` for path-based work
- `AnalyzePath` / analysis on handles — content-based MIME and type candidates
- Sync and async read, write, copy, move, rename, delete
- Directory watching with typed change events
- One NuGet with native assets for supported RIDs

## Install

**Prerequisites:** .NET SDK **10.0.x** and a supported RID at runtime.

```powershell
dotnet add package Dhara.Storage --version 0.9.0
```

| OS | RIDs |
|----|------|
| Windows | `win-x64`, `win-arm64` |
| Linux | `linux-x64`, `linux-arm64` |
| macOS | `osx-arm64` |

## Usage

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

Optional shell icons return raw RGBA pixels (`includeIcon: true`). `ShellDetails` is Windows-only today.

## Related

- Product overview: [repo root][repo-root]
- Native ABI: [dharastorage][repo-dharastorage]
- Typed ABI reference: [docs][typed-abi]

## License

Apache-2.0. Part of the [Dhara Storage workspace][repo-root].

[repo-root]: https://github.com/D-Naveenz/dhara_storage
[repo-dharastorage]: https://github.com/D-Naveenz/dhara_storage/tree/main/src/bindings/dharastorage-ffi
[typed-abi]: https://github.com/D-Naveenz/dhara_storage/blob/main/docs/typed-c-compatible-abi.md
