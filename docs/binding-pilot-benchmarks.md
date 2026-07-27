# Binding-transport pilot benchmarks

Windows-first groundwork comparing **in-process FFI** (B1, current `Dhara.Storage`) against a **Rust gRPC daemon** (B2) for foreign-language bindings. Rust apps continue to use `dhara_storage` directly.

This is a pilot—not a production migration. Results inform a later go / hybrid / stay-FFI decision ahead of `StorageProcess` / `ProcessingQueue` work. Benchmarks are **manual only** (not CI/CD).

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
dotnet run --project benchmark/Dhara.Storage.BenchPilot/Dhara.Storage.BenchPilot.csproj -c Release --no-build -- --filter *Read*
```

Harness uses `BenchmarkDotNet` 0.15.8 with `MemoryDiagnoser`. Supporting code: `DaemonHost`, `FixtureFactory`, `BindingBenchmarks`.

## Decision thresholds (vs B1)

| Scenario | Acceptable | Must match/beat B1 |
|----------|------------|--------------------|
| List entries (chatty) | ≤ +15% mean | — |
| Analyze small file | ≤ +10% | — |
| Read ≥1MB (handle dup) | — | Throughput / mean competitive with B1 |
| Copy 1MB | ≤ +10% | — |
| Queue stub | Document IPC suitability | — |

**Outcomes:** go daemon / hybrid / stay FFI.

## Showcase results (Release, full suite)

**Host:** Windows 11 (10.0.26200), AMD Ryzen 9 5900HS (16 logical / 8 physical cores), .NET 10.0.10, BenchmarkDotNet 0.15.8, `IterationCount=8`, `WarmupCount=1`.

| Method | Mean | Allocated | Notes |
|--------|------|-----------|-------|
| B2 Ping | 94 µs | ~6.6 KB | Control-plane floor |
| B2 Echo 1KB | 94 µs | ~8.4 KB | |
| B2 Echo 64KB | 292 µs | ~76 KB | |
| B1 GetFileInfo | 1.32 ms | ~0.6 KB | |
| B2 GetFileInfo | 159 µs | ~6.9 KB | B2 faster here (FFI path carries more overhead on this host) |
| B1 ListEntries/100 | 10.1 ms | ~34 KB | |
| B2 ListEntries/100 | 10.7 ms | ~43 KB | ≈ +5% vs B1 (within ≤ +15%) |
| B1 AnalyzePath (PDF) | 5.1 ms | ~0.5 KB | |
| B1 Read 4KB | 125 µs | ~4 KB | |
| B2 ReadHandleDup 4KB | 269 µs | ~72 KB | Dup overhead dominates tiny reads |
| B1 Read 1MB | 650 µs | ~1.0 MB | Full buffer over FFI |
| B2 ReadHandleDup 1MB | 321 µs | ~72 KB | **Beats B1**; host reads via duplicated handle |
| B2 ReadBytesGrpc 1MB | 2.84 ms | ~1.1 MB | Contrast: bytes-over-gRPC ~9× slower than handle-dup |
| B1 Write 1MB | 3.36 ms | ~0.4 KB | |
| B1 Copy 1MB | 78.5 ms | ~1.3 KB | |
| B2 QueueStub 20 jobs | 4.93 ms | ~142 KB | Synthetic enqueue + event stream |

**Reading:** handle duplication is the right data-plane for large B2 reads. Chatty list is competitive. Tiny B2 reads pay IPC/dup tax.

**Pilot lean:** **hybrid** — keep FFI (or tighten it) for small ops; use a daemon + handle transfer for large reads / isolation. Not a pure go-daemon cutover yet.

### Suite gaps / known hangers

These B2 methods hang under iterative BenchmarkDotNet on this workstation (named-pipe gRPC + streaming/large unary payloads). They remain implemented on the daemon but are **not** in the default BDN suite:

- B2 AnalyzePath
- B2 WriteFileBytes (1MB)
- B2 CopyFile (progress stream)

`DaemonHost` uses a single HTTP/2 pipe connection (`EnableMultipleHttp2Connections = false`). Heavy daemon handlers run under `spawn_blocking`. Further pipe/stream hardening is future work.

Cold start (spawn + first Ping) is dominated by process create + named-pipe connect; use Ping (~94 µs steady-state) as the warm control-plane floor rather than a separate cold-start job.

## Smoke snapshot (`--smoke`, Release)

Indicative only (short iteration counts):

| Method | Mean | Allocated |
|--------|------|-----------|
| Ping (B2) | ~146–276 µs | ~6.6 KB |
| GetFileInfo_B1 | ~1.3–1.5 ms | ~0.7 KB |
| GetFileInfo_B2 | ~257–304 µs | ~7 KB |

Prefer the showcase table above for decisions.

## Extension hooks (not in this milestone)

- Unix: UDS + `SCM_RIGHTS`
- Python: same daemon + `grpcio`
- Replace queue stubs with real `ProcessingQueue`
- Directory-watch event latency (awkward in BDN; add only if needed)
- Stabilize B2 analyze / large write / copy streams under BDN
