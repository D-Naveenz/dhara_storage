# AGENTS.md

Read this file before large changes in this workspace. It is the durable product note and AI/dev router for Dhara Storage.

This is a **hybrid** repository: product code lives in this git history; the operator tool is the [`tooling/drot`](tooling/drot) submodule. For DROT work, also follow [`tooling/drot/AGENTS.md`](tooling/drot/AGENTS.md) and its `docs/` / `.cursor/rules`.

## Human vs AI docs

| Surface | Audience | Role |
|---------|----------|------|
| `README.md` (root and **project** packages) | Humans | What / why / how to use — outcome language only |
| This file (`AGENTS.md`) | Humans + AI | Ambition, lineage, architecture, commands, guardrails |
| `docs/**` | Implementers | ABI, CI maps, DSFD, and other **storage** deep reference |
| [`tooling/drot/AGENTS.md`](tooling/drot/AGENTS.md) + [`tooling/drot/docs/`](tooling/drot/docs/) | Humans + AI | **DROT** tool intent, TUI/CLI, operator logging — owned by the submodule |

**Agents:** follow the global **`project-docs`** skill, then [`.cursor/rules/project-docs.mdc`](.cursor/rules/project-docs.mdc) (repo customizations) for README / AGENTS / `docs/`. For source **documentation comments**, headers, and why-comments, follow **`inline-code-docs`**. Project/registry READMEs have priority over the root README. No folder-container READMEs. Submodule project READMEs use the same convention.

**DROT work:** edit docs and Cursor rules **inside** [`tooling/drot`](tooling/drot). Do not add DROT-deep essays under storage `docs/` — link to the submodule instead.

---

## Product intent

### Ambition

Cross-platform **local** storage runtime with a handle-based file/directory model and first-class **content-based** file intelligence bundled with storage ops. Not a thin MIME add-on. Not cloud/object storage.

### Goals

- Usable from ordinary libraries and desktop apps (no OS-app / UWP / WinAppSDK lock-in)
- Same mental model on Windows, Linux, and macOS
- Transfers with progress and cancellation; directory watching; shell metadata where useful
- Rust core for memory safety, thread safety, and fearless concurrency under heavy I/O and analysis

### Design lineage (named here — not in READMEs)

Portable answer to `Windows.Storage` limits (WinRT-shaped APIs hard to reuse from a normal library or another OS). Microsoft Learn documents that [`StorageFile.ContentType`](https://learn.microsoft.com/uwp/api/windows.storage.storagefile.contenttype) is an **extension association**, not byte inspection. Dhara implements content signatures and the storage runtime in Rust — not a Windows.Storage mimic.

### Comparison (agents)

| Topic | Typical platform storage (`Windows.Storage`) | Dhara |
|-------|-----------------------------------------------|-------|
| Model | `StorageFile` / `StorageFolder` handles (often brokered) | `StorageFile` / `StorageDirectory` (Rust: `FileStorage` / `DirectoryStorage`) over ordinary absolute paths |
| “Content type” | Extension association (MS docs) | Opt-in content signatures via bundled `filedefs.dat` → ranked MIME / type |
| Platform | Windows / WinRT sandbox & pickers | Multi-RID desktop; usable from libraries |
| Transfers / watch | Uneven progress; watch often external | Progress + cancel; debounced directory watch; shell icon RGBA (+ Windows shell details) |

### Intentionally omitted from READMEs

Agents editing READMEs must **not** add:

- “Inspired by Windows.Storage” or UWP / WinAppSDK war stories
- Version number as the hero pitch
- Stack-first / “Rust-first / Windows-first delivery” framing
- NuGet / C ABI / monorepo trees as the primary story
- Operator / `drot` / CI / env-secret essays (link [`tooling/drot/`](tooling/drot/) or this file instead)
- Architecture dumps, implementation logs, or creation history

### Locked human pitch (source of truth for README leads)

> Dhara Storage is a cross-platform local storage runtime for applications. You work with file and directory handles—read, write, copy, move, watch, and metadata—and when you need to know what a file really is, you get **content-based type intelligence**, not a guess from the extension.
>
> Ordinary file APIs stop at paths and bytes. Platform “content type” fields often stop at the name. Dhara combines a storage-handle model with signature-based analysis in one runtime, so business apps and libraries can classify and manage files without a separate MIME stack—and without being locked to one OS app model.
>
> The core is written in Rust for memory safety, thread safety, and fearless concurrency under heavy I/O and analysis workloads.

**Why-you-might-use-it (human bullets):**

- Know the real type from file bytes (bundled definitions), not just the extension
- File and directory handles with transfers that support progress and cancellation
- Built-in directory watching and shell-aware metadata where the OS provides it
- Use it from libraries and desktop apps on Windows, Linux, and macOS—same idea everywhere
- Rust core when concurrency and system-level correctness matter

---

## Architecture map

| Path | Role |
|------|------|
| `core/dhara_storage` | Business runtime — analysis, storage handles, ops, watching, metadata; embeds `filedefs.dat` |
| `core/dhara_storage_core` | Framework layer — DSFD, process (progress/cancel), portable types/options; planned: `StorageProcess` / `ProcessingQueue` |
| `interop/dhara-sd` | Sidecar daemon (`dhara-sd`) — gRPC control + handle transfer for foreign bindings |
| `interop/dharastorage-ffi` | C ABI (`dharastorage` cdylib) — **benchmark / evidence only**; not shipped via NuGet |
| `bindings/csharp/Dhara.Storage` | .NET 10 NuGet — managed API over `dhara-sd` |
| `bindings/csharp/Dhara.Storage.Extensions.Hosting` | .NET 10 NuGet — Generic Host lifetime for the sidecar |
| `benchmark/Dhara.Storage.Benchmarks` | Manual BenchmarkDotNet harness (FFI vs daemon rung 1; not CI/CD) |
| `tooling/drot` | Submodule ([dhara_repo_orchestration](https://github.com/D-Naveenz/dhara_repo_orchestration)) — operator CLI/TUI; **owns** its [AGENTS.md](tooling/drot/AGENTS.md) and [docs/](tooling/drot/docs/) |
| `dhara.config.toml` | Shared product versions, NuGet metadata, RIDs (not DROT tool version) |

**Design choices**

- `dhara_storage_core` is the framework; `dhara_storage` holds business process. Shipped core slices: **DSFD**, **process** (progress/cancel/reporter), and **types** (options + portable attribute/permission/size shapes). Next planned slice: `StorageProcess` / `ProcessingQueue` on top of `process`.
- Concrete `FileStorage` / `DirectoryStorage` stay in the runtime. Extension crates (for example a future archives crate) may depend on `dhara_storage`, compose those handles, and define their own types. No product-level storage-object enum or `dyn` handle trait in core — closed mixed collections are consumer-owned enums.
- Keep `dhara_storage` Rust-native; foreign hosts use **`dhara-sd`** (not in-process FFI for the NuGet).
- Windows is the primary **developer workstation**; ship all five 64-bit RIDs via CI (`package stage-native` per OS + `native merge`).
- Product / NuGet semver: [`dhara.config.toml`](dhara.config.toml) `[versions]`. DROT tool version: [`tooling/drot/Cargo.toml`](tooling/drot/Cargo.toml). Storage pins the tool via submodule gitlink.

Deep reference: [docs/README.md](docs/README.md).

---

## Local commands

- Init submodule: `git submodule update --init --recursive`
- Ensure production-shaped CLI/TUI: `./tooling/scripts/run-drot.ps1` (no subcommand opens **TUI**; pass a subcommand or `--help` for the Direct CLI; use `-Force` / `--force-build` to rebuild from submodule). Dist is gated by `tooling/drot` git `HEAD` via `target/dist/.drot-git-rev` (not Cargo semver).
- Full local check (CI parity): `./tooling/scripts/verify-local.ps1` — runs `run-drot --yes quality run`
- Full repository build (CLI): `./tooling/scripts/run-drot.ps1 --yes build run` (config → defs → quality → native → verify; skip flags available)
- Windows GitHub SSH + LFS: `./tooling/scripts/setup-github-ssh.ps1` (analyze by default; `-Repair` or `-Recreate` to act)
- Active DROT development: work in the orchestration repo (or `tooling/drot` submodule); follow [tooling/drot/AGENTS.md](tooling/drot/AGENTS.md). Local: `cargo build --manifest-path tooling/drot/Cargo.toml -p drot --profile dist`
- Verify NuGet package shape: `target/dist/drot -r . --yes verify package` (after ensure)
- **Tool version:** owned only by DROT (`tooling/drot/Cargo.toml`). Storage pins via submodule gitlink. Workspace/NuGet manifest drift reconciles on the next `drot` run (confirm activation, or `--yes` in CI/scripts).

**DROT source vs artifacts (agents):** edit sources under [`tooling/drot`](tooling/drot); run via `./tooling/scripts/run-drot.ps1` (sets `CARGO_TARGET_DIR` to this repo’s `target/`). The `drot` binary and `.drot-git-rev` live at **`target/dist/`** on the host — not inside the submodule. When spawning subagents for DROT work, pass both paths so they do not hunt for `drot.exe` under `tooling/drot`. DROT links one compile-time **extension** (`drot_dhara_storage` by default); see [tooling/drot/AGENTS.md](tooling/drot/AGENTS.md).

### Release / env / CI

Shared metadata and secrets: [dhara.config.toml](dhara.config.toml), [`.env.example`](.env.example), and DROT [host-config](tooling/drot/docs/host-config.md). Pipeline map, Environments, Dependabot, and publish jobs: [docs/ci-cd-pipelines.md](docs/ci-cd-pipelines.md). Workflows: [`.github/workflows/quality.yml`](.github/workflows/quality.yml), [`.github/workflows/package-pipeline.yml`](.github/workflows/package-pipeline.yml), [`publish-crates.yml`](.github/workflows/publish-crates.yml), [`publish-nuget.yml`](.github/workflows/publish-nuget.yml). Local DROT: [`run-drot`](tooling/scripts/run-drot.ps1); CI downloads prebuilt `drot` for the pinned submodule SHA ([`download-drot`](.github/actions/download-drot/action.yml)).

---

## Guardrails

- Keep `dhara_storage` Rust-native; foreign binding interop is the `dhara-sd` daemon (+ Hosting). Keep `dharastorage-ffi` only for evidence benchmarks until retired.
- Treat Windows as the primary workstation; ship all five 64-bit RIDs via CI merge.
- When rewriting README marketing, use **Product intent → Locked human pitch** above — do not invent a new story.
- Do not add local private paths to this file.
- Prefer clean-cut breaking changes; decide in plan / follow plan / ask when unclear — see [`.cursor/rules/breaking-changes.mdc`](.cursor/rules/breaking-changes.mdc).
