# Binding benchmarks (daemon vs FFI)

Manual BenchmarkDotNet suite comparing **legacy in-process FFI** (evidence baseline) against **`dhara-sd` + handle duplication** (product path). Rust apps continue to use `dhara_storage` directly.

Not CI/CD. Windows-first.

## Benchmark ladder

| Rung | Audience | Comparison | Status |
|------|----------|------------|--------|
| **1** | “Why drop FFI from NuGet?” | `dharastorage-ffi` (bench harness) vs `dhara-sd` + handle-dup | **Current suite** |
| **2** | Developers | Daemon RPC delay vs in-process `dhara_storage` (Criterion) | Planned |
| **3** | Users | `Dhara.Storage` vs language-native I/O (e.g. `File.Copy`) | Planned |

## Layout

| Path | Role |
|------|------|
| [`interop/proto/dhara_sd.proto`](../interop/proto/dhara_sd.proto) | gRPC contract (`dhara.sd.v1`) |
| [`interop/dhara-sd/`](../interop/dhara-sd/) | Sidecar daemon |
| [`interop/dharastorage-ffi/`](../interop/dharastorage-ffi/) | FFI cdylib for rung 1 only |
| [`benchmark/Dhara.Storage.Benchmarks/`](../benchmark/Dhara.Storage.Benchmarks/) | BenchmarkDotNet harness |

Outputs: `target/bench-pilot/bdn/` (gitignored; path may rename later).

## Baselines (rung 1)

| ID | Stack |
|----|--------|
| **B1** | C# bench → `dharastorage` cdylib (not via NuGet) |
| **B2** | C# → gRPC named pipe → `dhara-sd` → handle-dup / control RPCs |

Product data plane: **handle transfer**, not bytes-over-gRPC. `ReadFileBytes` / `WriteFileBytes` are contrast-only.

## How to run

```powershell
dotnet build benchmark/Dhara.Storage.Benchmarks/Dhara.Storage.Benchmarks.csproj -c Release
dotnet run --project benchmark/Dhara.Storage.Benchmarks/Dhara.Storage.Benchmarks.csproj -c Release --no-build
dotnet run --project benchmark/Dhara.Storage.Benchmarks/Dhara.Storage.Benchmarks.csproj -c Release --no-build -- --smoke
dotnet run --project benchmark/Dhara.Storage.Benchmarks/Dhara.Storage.Benchmarks.csproj -c Release --no-build -- --filter *Read*
```

## Decision thresholds (vs B1)

| Scenario | Acceptable | Must match/beat B1 |
|----------|------------|--------------------|
| List entries (chatty) | ≤ +15% mean | — |
| Analyze small file | ≤ +10% | — |
| Read ≥1MB (handle dup) | — | Throughput / mean competitive with B1 |
| Copy 1MB | ≤ +10% | — |

**Product lean:** NuGet uses `dhara-sd`; keep FFI only until rung-1 evidence is archived.

## Stability notes

1. Daemon accept loop creates the next listening pipe before yield.
2. Host must not redirect unread daemon stderr.
3. Client: single HTTP/2 pipe connection; absolute gRPC deadlines.
4. `CPUUsageDiagnoser` may be commented when DiagnosticsHub ETW is exhausted.

Transport design: [daemon-transport.md](daemon-transport.md). Signing: [windows-code-signing.md](windows-code-signing.md).

## Showcase (last full Release run)

Re-run the suite after significant transport or packaging changes and refresh this section with current numbers.
