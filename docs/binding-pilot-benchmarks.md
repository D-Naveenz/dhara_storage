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

Harness uses `BenchmarkDotNet` 0.15.8 with `MemoryDiagnoser`. `CPUUsageDiagnoser` (DiagnosticsHub package) is present but commented out when ETW sessions are exhausted on the workstation. Supporting code: `DaemonHost`, `FixtureFactory`, `BindingBenchmarks`.

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
| B2 Ping | 181 µs | ~7 KB | Control-plane floor |
| B2 Echo 1KB | 222 µs | ~9 KB | |
| B2 Echo 64KB | 518 µs | ~76 KB | |
| B1 GetFileInfo | 1.94 ms | ~0.6 KB | |
| B2 GetFileInfo | 250 µs | ~7 KB | B2 faster here (FFI path carries more overhead on this host) |
| B1 ListEntries/100 | 13.3 ms | ~33 KB | |
| B2 ListEntries/100 | 20.3 ms | ~48 KB | ≈ +53% vs B1 (above ≤ +15% chatty threshold on this run) |
| B1 AnalyzePath (PDF) | 6.4 ms | ~0.5 KB | |
| B2 AnalyzePath (PDF) | 9.4 ms | ~10 KB | ≈ +45% vs B1 |
| B1 Read 4KB | 161 µs | ~4 KB | |
| B2 ReadHandleDup 4KB | 617 µs | ~71 KB | Dup overhead dominates tiny reads |
| B1 Read 1MB | 814 µs | ~1.0 MB | Full buffer over FFI |
| B2 ReadHandleDup 1MB | 778 µs | ~71 KB | Competitive with B1; host reads via duplicated handle |
| B2 ReadBytesGrpc 1MB | 6.7 ms | ~2.1 MB | Contrast: bytes-over-gRPC much slower than handle-dup |
| B1 Write 1MB | 5.9 ms | ~0.4 KB | |
| B2 Write 1MB | 11.1 ms | ~154 KB | ≈ +89% vs B1 |
| B1 Copy 1MB | 78.4 ms | ~1.3 KB | |
| B2 Copy 1MB | 5.9 ms | ~84 KB | **Beats B1** (~8×) on this host |
| B2 QueueStub 20 jobs | 5.3 ms | ~149 KB | Synthetic enqueue + event stream |

**Reading:** handle duplication remains the right data-plane for large B2 reads. B2 copy streaming wins clearly vs B1 on this machine. Chatty list and analyze still pay IPC tax.

**Pilot lean:** **hybrid** — keep FFI (or tighten it) for small ops; use a daemon + handle transfer for large reads / isolation; copy-over-RPC is promising. Not a pure go-daemon cutover yet.

### Stability notes

Named-pipe hang fixes in this pilot:

1. Daemon accept loop creates the **next** listening pipe instance **before** yielding a connected one (Tokio Windows named-pipe pattern).
2. `DaemonHost` must **not** redirect unread daemon stdout/stderr — a full stderr pipe blocked the daemon under `info`-level Analyze logging.
3. Client: single HTTP/2 pipe connection (`EnableMultipleHttp2Connections = false`), multi-MB message size limits, absolute gRPC deadlines (not GC-discarded `CancellationTokenSource` tokens).

Heavy daemon handlers still use `spawn_blocking`. Cold start (spawn + first Ping) is dominated by process create + named-pipe connect; use Ping as the warm control-plane floor.

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
