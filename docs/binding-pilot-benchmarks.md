# Binding-transport pilot benchmarks

Windows-first groundwork comparing **in-process FFI** (B1, current `Dhara.Storage`) against a **Rust gRPC daemon** (B2) for foreign-language bindings. Rust apps continue to use `dhara_storage` directly.

This is a pilot—not a production migration. Results inform a later go / hybrid / stay-FFI decision ahead of `StorageProcess` / `ProcessingQueue` work.

## Layout

| Path | Role |
|------|------|
| [`interop/proto/dhara_pilot.proto`](../interop/proto/dhara_pilot.proto) | gRPC contract (`dhara.pilot.v1`) |
| [`interop/dhara-storage-daemon-pilot/`](../interop/dhara-storage-daemon-pilot/) | Windows named-pipe tonic server |
| [`benchmark/Dhara.Storage.BenchPilot/`](../benchmark/Dhara.Storage.BenchPilot/) | BenchmarkDotNet harness |

Outputs: `target/bench-pilot/bdn/` (gitignored).

## Baselines

| ID | Stack |
|----|--------|
| **B1** | C# → `Dhara.Storage` → `dharastorage` cdylib |
| **B2** | C# → gRPC named pipe → `dhara-storage-daemon-pilot` → `dhara_storage` |

## How to run

```powershell
dotnet build benchmark/Dhara.Storage.BenchPilot/Dhara.Storage.BenchPilot.csproj -c Release

# Full B1/B2 suite (BenchmarkDotNet)
dotnet run --project benchmark/Dhara.Storage.BenchPilot/Dhara.Storage.BenchPilot.csproj -c Release --no-build

# Short smoke suite
dotnet run --project benchmark/Dhara.Storage.BenchPilot/Dhara.Storage.BenchPilot.csproj -c Release --no-build -- --smoke

# Filter methods (BenchmarkDotNet args)
dotnet run --project benchmark/Dhara.Storage.BenchPilot/Dhara.Storage.BenchPilot.csproj -c Release --no-build -- --filter *Copy*
```

Packages match the Visual Studio BenchmarkSuite template: `BenchmarkDotNet` 0.15.8 + DiagnosticsHub diagnosers (`MemoryDiagnoser`, `CPUUsageDiagnoser`).

Supporting code kept intentionally small: `DaemonHost` (spawn/handshake), `FixtureFactory` (test files), `BindingBenchmarks` (measurements). No custom timing/report harness.

## Decision thresholds (vs B1)

| Scenario | Acceptable | Must match/beat B1 |
|----------|------------|--------------------|
| List entries (chatty) | ≤ +15% mean | — |
| Analyze small file | ≤ +10% | — |
| Read ≥1MB (handle dup) | — | Throughput / mean competitive with B1 |
| Copy 1MB | ≤ +10% | — |
| Queue stub | Document IPC suitability | — |

**Outcomes:** go daemon / hybrid / stay FFI.

## Smoke snapshot (`--smoke`, Release)

| Method | Mean | Allocated |
|--------|------|-----------|
| Ping (B2) | ~276 µs | ~6.6 KB |
| GetFileInfo_B1 | ~1.54 ms | ~688 B |
| GetFileInfo_B2 | ~257 µs | ~7.2 KB |

Run the full suite (no `--smoke`) for decision-grade iteration counts.

## Extension hooks (not in this milestone)

- Unix: UDS + `SCM_RIGHTS`
- Python: same daemon + `grpcio`
- Replace queue stubs with real `ProcessingQueue`
- Directory-watch event latency (awkward in BDN; add only if needed)
