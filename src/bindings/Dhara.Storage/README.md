# Dhara.Storage for .NET

`Dhara.Storage` is the `net10.0` managed wrapper over the native `dharastorage`
runtime.

It gives .NET applications an object-oriented, path-based API for the Rust core
without pushing .NET object-shape requirements back into the native runtime.

The current NuGet publish target is `Dhara.Storage` `0.7.0`.

## Supported Platforms

`Dhara.Storage` uses a native Rust backend. The managed assembly is platform
neutral in principle, but a matching native asset must exist for the runtime.

The current NuGet package ships native assets for:

- Windows `win-x64`
- Windows `win-arm64`

Unsupported platforms are rejected in two places:

- at package-consumption time through a transitive `.targets` file for 32-bit runtime identifiers, 32-bit build platforms, and `Prefer32Bit=true`
- at runtime through a managed `PlatformNotSupportedException` guard

## Install

```bash
dotnet add package Dhara.Storage --version 0.7.0
```

## Quick Start

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

## Public API

- `DharaStorage.File(path)` creates a `StorageFile`
- `DharaStorage.Directory(path)` creates a `StorageDirectory`
- `DharaStorage.AnalyzePath(path)` runs point-in-time analysis without creating a wrapper object
- `StorageFile` exposes sync and async methods for analysis, reads, writes, copy, move, rename, and delete
- `StorageDirectory` exposes enumeration, create, copy, move, rename, delete, and explicit watching

## Logging

`Dhara.Storage` integrates with `Microsoft.Extensions.Logging`.

```csharp
using var loggerFactory = LoggerFactory.Create(builder =>
{
    builder.AddConsole();
    builder.SetMinimumLevel(LogLevel.Debug);
});

DharaStorage.UseLoggerFactory(loggerFactory);
```

Once configured, the host receives:

- managed wrapper logs from async handles and orchestration code
- native Rust logs forwarded from `tracing` through the FFI logger bridge

## Packaging Notes

- Local `dotnet build` copies the native DLL into the output folder for development.
- `dotnet pack` is intentionally guarded so local packing cannot silently create a misleading single-runtime package.
- Repository packaging flows stage both native assets before packing:
  - `runtimes/win-x64/native/dharastorage.dll`
  - `runtimes/win-arm64/native/dharastorage.dll`
