# Binding benchmarks (daemon vs FFI)

Manual BenchmarkDotNet suite comparing **legacy in-process FFI** (evidence baseline) against **`dhara-sd` + handle duplication** (product path). Rust apps continue to use `dhara_storage` directly.

Not CI/CD. Windows-first.

## Benchmark ladder

| Rung | Audience | Comparison | Status |
|------|----------|------------|--------|
| **1** | “Why drop FFI from NuGet?” | `dharastorage-ffi` (bench harness) vs `dhara-sd` + handle-dup | **Current suite** (decision archived; suite kept for regression) |
| **2** | Developers | Daemon RPC delay vs in-process `dhara_storage` (Criterion) | Planned |
| **3** | Users | `Dhara.Storage` / daemon vs a **C# approximation** of Dhara features (progress, cancellation, etc.—what managed code can fairly recreate; not bare BCL one-liners) | Planned |

Rung 1 answered the NuGet packaging question. Future **user-facing** evidence is rung 3: show why the product path is worth adopting versus building a partial managed equivalent yourself—not versus `File.Copy` alone (Dhara does more than a thin copy).

## Layout

| Path | Role |
|------|------|
| [`interop/proto/dhara_sd.proto`](../interop/proto/dhara_sd.proto) | gRPC contract (`dhara.sd.v1`) |
| [`interop/dhara-sd/`](../interop/dhara-sd/) | Sidecar daemon |
| [`interop/dharastorage-ffi/`](../interop/dharastorage-ffi/) | FFI cdylib for rung 1 only |
| [`benchmark/Dhara.Storage.Benchmarks/`](../benchmark/Dhara.Storage.Benchmarks/) | BenchmarkDotNet harness |

Outputs (gitignored): `target/benchmarks/` — fixtures under `fixtures/`, BenchmarkDotNet artifacts under `bdn/`.

## Reports

After a run, open session-stamped files under `target/benchmarks/bdn/results/` (same timestamp stem as the BDN `.log` in `bdn/`):

| Pattern | Audience | Contents |
|---------|----------|----------|
| `{suite}-{yyyyMMdd-HHmmss}-report.html` | Humans | Styled evidence report: purpose, how we measure / host specs, findings + B1/B2 comparison, detailed table, footer |
| `{suite}-{yyyyMMdd-HHmmss}-results.json` | Machines / agents | Raw per-method stats plus comparison verdicts (`dhara.benchmarks.results/v1`) |

Each run keeps its own pair; nothing is overwritten by the next session.

Default BenchmarkDotNet CSV/HTML/Markdown sprawl is disabled. Paste findings from `report.html` into **Showcase** below when you want docs to carry a snapshot.

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

The HTML report applies these budgets to paired scenarios (Pass / Watch / Fail). Unpaired B2-only methods appear without ratios.

**Product lean:** NuGet uses `dhara-sd`; FFI remains bench-only for rung-1 regression.

## Stability notes

1. Daemon accept loop creates the next listening pipe before yield.
2. Host must not redirect unread daemon stderr.
3. Client: single HTTP/2 pipe connection; absolute gRPC deadlines.
4. `CPUUsageDiagnoser` may be commented when DiagnosticsHub ETW is exhausted.

Transport design: [daemon-transport.md](daemon-transport.md). Signing: [windows-code-signing.md](windows-code-signing.md).

## Showcase (last full Release run)

Re-run the suite after significant transport or packaging changes and refresh this section from `report.html` findings.
