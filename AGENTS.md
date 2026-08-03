# AGENTS.md

Read this file before large changes in this workspace. It is the durable product note and AI/dev router for Dhara Storage. MindVault is **not** used here.

## Human vs AI docs

| Surface | Audience | Role |
|---------|----------|------|
| `README.md` (root and **project** packages) | Humans | What / why / how to use — outcome language only |
| This file (`AGENTS.md`) | Humans + AI | Ambition, lineage, architecture, commands, CI, guardrails |
| `docs/**` | Implementers | ABI, CI maps, DSFD, and other **storage** deep reference |
| [`tooling/drot/AGENTS.md`](tooling/drot/AGENTS.md) + [`tooling/drot/docs/`](tooling/drot/docs/) | Humans + AI | **DROT** tool intent, TUI/CLI, operator logging — owned by the submodule |

**Agents:** follow the global **`project-docs`** skill, then [`.cursor/rules/project-docs.mdc`](.cursor/rules/project-docs.mdc) (repo customizations) for README / AGENTS / `docs/`. For source **documentation comments**, headers, and why-comments, follow **`inline-code-docs`**. Project/registry READMEs have priority over the root README. No folder-container READMEs. Submodule project READMEs use the same convention.

**DROT work:** edit docs and Cursor rules **inside** [`tooling/drot`](tooling/drot) (orchestration repo). Do not add DROT-deep essays under storage `docs/` — link to the submodule instead.

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

Started as a portable answer to `Windows.Storage` (`StorageFile` / `StorageFolder`) limits: those APIs are WinRT/Windows-shaped (capabilities, pickers, known folders, app data) and hard to reuse from a normal library or another OS.

Microsoft Learn documents that [`StorageFile.ContentType`](https://learn.microsoft.com/uwp/api/windows.storage.storagefile.contenttype) is an **extension association** — it does not inspect bytes (rename `.txt` → `.jpg` → reports `image/jpeg`).

A pure C# mimic hit concurrency and system-level limits. The conclusion: implement content signature definitions and the storage runtime in Rust. Today Dhara is **not** a Windows.Storage mimic — it is the stronger, unrestricted model.

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
| `core/dhara_storage_core` | Framework / abstraction layer for the runtime (DSFD definitions today; planned: process/queue primitives) |
| `interop/dhara-sd` | Sidecar daemon (`dhara-sd`) — gRPC control + handle transfer for foreign bindings |
| `interop/dharastorage-ffi` | C ABI (`dharastorage` cdylib) — **benchmark / evidence only**; not shipped via NuGet |
| `bindings/csharp/Dhara.Storage` | .NET 10 NuGet — managed API over `dhara-sd` |
| `bindings/csharp/Dhara.Storage.Extensions.Hosting` | Generic Host lifetime for the sidecar |
| `benchmark/Dhara.Storage.Benchmarks` | Manual BenchmarkDotNet harness (FFI vs daemon rung 1; not CI/CD) |
| `tooling/drot` | Submodule ([dhara_repo_orchestration](https://github.com/D-Naveenz/dhara_repo_orchestration)) — operator CLI/TUI; **owns** its [AGENTS.md](tooling/drot/AGENTS.md) and [docs/](tooling/drot/docs/) |
| `dhara.config.toml` | Shared product versions, NuGet metadata, RIDs (not DROT tool version) |

**Design choices**

- `dhara_storage_core` is the framework; `dhara_storage` holds business process. DSFD is the first shipped core slice — not the whole story. Planned core additions include primitives such as `StorageProcess` and `ProcessingQueue` (not shipped yet).
- Keep `dhara_storage` Rust-native; foreign hosts use **`dhara-sd`** (not in-process FFI for the NuGet).
- Windows is the primary **developer workstation**; ship all five 64-bit RIDs via CI (`package stage-native` per OS + `native merge`).
- Current product line: **0.9.22** (workspace crates and NuGet). `drot` is independently versioned in the DROT submodule (**0.9.13** in `tooling/drot/Cargo.toml`).

Deep reference: [docs/README.md](docs/README.md).

---

## Local commands

- Init submodule: `git submodule update --init --recursive`
- Ensure production-shaped CLI/TUI: `./tooling/scripts/run-drot.ps1` (default opens **TUI**; pass `-Cli` / `--cli` for the direct CLI; use `-Force` / `--force-build` to rebuild from submodule)
- Full local check (CI parity): `./tooling/scripts/verify-local.ps1` — runs `run-drot -Cli --yes quality run`
- Full repository build (CLI): `./tooling/scripts/run-drot.ps1 -Cli --yes build run` (config → defs → quality → native → verify; skip flags available)
- Windows GitHub SSH + LFS: `./tooling/scripts/setup-github-ssh.ps1` (analyze by default; `-Repair` or `-Recreate` to act)
- Active DROT development: work in the orchestration repo (or `tooling/drot` submodule); follow [tooling/drot/AGENTS.md](tooling/drot/AGENTS.md). Build with `cargo build --manifest-path tooling/drot/Cargo.toml -p drot`
- Verify NuGet package shape: `target/dist/drot -r . --yes verify package` (after ensure)
- **Tool version:** owned only by DROT (`tooling/drot/Cargo.toml`). Storage pins via submodule gitlink. Workspace/NuGet manifest drift reconciles on the next `drot` run (confirm activation, or `--yes` in CI/scripts).

**DROT rollout (orchestration repo):** commit submodule changes (core dep, `build run`, dependabot), merge on `dhara_repo_orchestration`, wait for `pack-windows` / `pack-linux` artifacts, then bump the storage `tooling/drot` gitlink to that SHA. Publish `dhara_storage_core` **0.9.22** to crates.io before pinning DROT to semver `0.9.22` if desired.

### Release / env (operator)

Shared metadata: [dhara.config.toml](dhara.config.toml). Local secrets in `.env.local` (from `.env.example`), not git.

CI credentials live on **GitHub Environments** (`staging`, `release-nuget`, `release-cargo`) — not repository Actions secrets/variables. Ensure-branch / Dependabot auto-merge / CodeQL use only the built-in Actions `github.token` (no Environment).

| Variable | Purpose | Where (CI) |
|----------|---------|------------|
| `NUGET_USER` | nuget.org profile name for OIDC login | GitHub Environment **variable** on `release-nuget` |
| `NUGET_SOURCE` | NuGet push feed URL (optional override) | GitHub Environment **variable** on `release-nuget`; else `dhara.config.toml` |
| `NUGET_API_KEY` | NuGet.org publish fallback | GitHub Environment `release-nuget` — used only if OIDC publish fails for a non-duplicate reason |
| `CARGO_REGISTRY_TOKEN` | crates.io publish fallback | GitHub Environment `release-cargo` — used only if OIDC publish fails for a non-duplicate reason |
| `TOOL_MAX_WORKERS` | Caps Rayon workers in `drot` defs builds | local / CI env as needed |
| `DROT_ARTIFACTS_TOKEN` | Download prebuilt `drot` from orchestration | GitHub Environment `staging` |

CI publish auth: try [NuGet](https://learn.microsoft.com/en-us/nuget/nuget-org/trusted-publishing) / [crates.io](https://crates.io/docs/trusted-publishing) Trusted Publishing (OIDC) first. Already-published versions exit successfully. Other failures fall back to the long-lived API key/token when present. Leave crates.io **Require trusted publishing** unchecked while fallback tokens may still be needed.

Scaffold env: `cargo run --manifest-path tooling/drot/Cargo.toml -p drot -- config env init`.

---

## CI/CD

- PR pipeline: [`.github/workflows/pipeline.yml`](.github/workflows/pipeline.yml) — environment `staging` — [docs/ci-cd-pipelines.md](docs/ci-cd-pipelines.md)
- CodeQL SAST: [`.github/workflows/codeql.yml`](.github/workflows/codeql.yml) — separate from Pipeline (fmt/clippy/doc stay in quality); no GitHub Environment; see [docs/ci-cd-pipelines.md](docs/ci-cd-pipelines.md)
- Branch / Dependabot: feature → `development` → `main`; version updates target `development` ([`dependabot.yml`](.github/dependabot.yml)), with ensure-branch + squash auto-merge workflows — details in [docs/ci-cd-pipelines.md](docs/ci-cd-pipelines.md) (**Branch flow and Dependabot**). Never auto-merge PRs into `main`.
- Merge publishes: [`publish-crates.yml`](.github/workflows/publish-crates.yml) (`release-cargo`, OIDC + optional bootstrap token), [`publish-nuget.yml`](.github/workflows/publish-nuget.yml) (`release-nuget`, OIDC) — path-filtered; `workflow_dispatch` when automation skips
- **PR quality** uses direct `cargo fmt/clippy/doc` (core + FFI only). **DROT** is downloaded as an artifact from `dhara_repo_orchestration` for the pinned submodule SHA ([`download-drot`](.github/actions/download-drot/action.yml)); requires secret `DROT_ARTIFACTS_TOKEN` on `staging`.
- CD on merge reuses PR artifacts; use merge commits (not squash) so NuGet CD can resolve `HEAD^2`.
- **DROT ↔ core:** `drot_dhara_storage` depends on published **`dhara_storage_core`** (crates.io; no `dhara_storage_dal`). `filedefs.dat` lives only under `core/dhara_storage/resources/` and is resolved at runtime once `-r` points at the storage repo. Storage CI downloads prebuilt `drot` from `dhara_repo_orchestration` for the pinned submodule SHA ([`download-drot`](.github/actions/download-drot/action.yml)); local dev uses [`run-drot`](tooling/scripts/run-drot.ps1).

---

## Guardrails

- Keep `dhara_storage` Rust-native; foreign binding interop is the `dhara-sd` daemon (+ Hosting). Keep `dharastorage-ffi` only for evidence benchmarks until retired.
- Treat Windows as the primary workstation; ship all five 64-bit RIDs via CI merge.
- When rewriting README marketing, use **Product intent → Locked human pitch** above — do not invent a new story.
- Do not add local private paths to this file.
- Breaking changes are acceptable pre-1.0; see [`.cursor/rules/breaking-changes.mdc`](.cursor/rules/breaking-changes.mdc).
