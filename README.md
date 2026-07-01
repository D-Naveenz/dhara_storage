# Dhara Storage

[![dhara_storage on crates.io](https://img.shields.io/crates/v/dhara_storage?label=dhara_storage)](https://crates.io/crates/dhara_storage)
[![dhara_storage_dal on crates.io](https://img.shields.io/crates/v/dhara_storage_dal?label=dhara_storage_dal)](https://crates.io/crates/dhara_storage_dal)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE.txt)

Dhara Storage is a Rust-first storage and file-analysis workspace with a Windows-first delivery story.
It ships a native runtime, a C ABI layer, a .NET NuGet package, and operator tooling from one repo.
Current release line: **0.8.0** (shared across crates and NuGet).

## ✨ Key Features

- **Definition-driven analysis** — content-based file typing via bundled `filedefs.dat`
- **Path-based storage API** — files, directories, copy/move/delete, optional progress
- **Debounced watching** — stable directory change notifications
- **Layered delivery** — Rust core → C ABI → `net10.0` managed wrapper
- **Multi-RID NuGet** — `win-x64`, `win-arm64`, `linux-x64`, `linux-arm64`, `osx-arm64`
- **Operator CLI** — config activation, native staging, package verify, and release flows

## 📦 Tech Stack & Architecture

| Layer | Technology |
|-------|------------|
| Core runtime | Rust (edition 2024), `tracing` |
| Definitions DAL | FlatBuffers, embedded `filedefs.dat` |
| Native interop | `cdylib` C ABI (`dharastorage`) |
| Managed bindings | .NET 10 (`Dhara.Storage`) |
| Operator surface | `dhara_tool` (Clap + iced GUI) |
| CI / release | GitHub Actions, `dhara.config.toml` |

```
dhara_storage/
├── src/
│   ├── core/
│   │   ├── dhara_storage/       # Rust runtime (crates.io)
│   │   └── dhara_storage_dal/   # FlatBuffers DAL (crates.io)
│   ├── bindings/
│   │   ├── dharastorage-ffi/    # C ABI (`dharastorage` cdylib)
│   │   └── csharp/              # Dhara.Storage NuGet source
├── tooling/
│   ├── dhara_tool/              # Operator CLI (nested workspace)
│   ├── scripts/                 # ensure-dhara-tool-dist, verify-local (dist quality run)
│   └── output/                  # staged packages (gitignored)
├── docs/                        # technical reference
├── dhara.config.toml            # shared version + publish metadata
└── .env.local                   # local secrets (from .env.example)
```

| Package | README | Publish surface |
|---------|--------|-----------------|
| `dhara_storage` | [crate readme][readme-dhara-storage] | crates.io |
| `dhara_storage_dal` | [crate readme][readme-dal] | crates.io |
| `dharastorage` | [crate readme][readme-dharastorage] | native asset in NuGet |
| `Dhara.Storage` | [package readme][readme-nuget] | NuGet.org |
| `dhara_tool` | [tool readme][readme-tool] | workspace-only |

## 🚀 Getting Started & Installation

**Prerequisites**

- Rust **stable** toolchain (`cargo`, `rustfmt`, `clippy`)
- .NET SDK **10.0.x** (for bindings tests and local .NET dev)
- PowerShell or bash (for [verify-local][verify-local])
- Windows: MSVC build tools when compiling `win-x64` / `win-arm64` natives locally

**Setup**

1. Clone the repository.
2. Copy [.env.example][env-example] to `.env.local` and fill publish keys only when releasing.
3. Run the local verify script from the repo root:

```powershell
./tooling/scripts/verify-local.ps1
```

## 🔧 Configuration & Environment Variables

Shared release metadata lives in [dhara.config.toml][dhara-config] (versions, NuGet IDs, native RIDs).

| Variable | Example | Purpose |
|----------|---------|---------|
| `CARGO_REGISTRY_TOKEN` | *(secret)* | crates.io publish (`release run`) |
| `NUGET_API_KEY` | *(secret)* | NuGet.org publish |
| `NUGET_SOURCE` | `https://api.nuget.org/v3/index.json` | NuGet feed URL |
| `TOOL_MAX_WORKERS` | `4` | Caps Rayon workers in `dhara_tool` defs builds |

Local secrets belong in `.env.local`, not in git. Run `cargo run -p dhara_tool -- config env init` to scaffold from the example file.

## 🛠️ Usage Examples

**Rust** — add [dhara_storage][readme-dhara-storage] to `Cargo.toml`:

```toml
[dependencies]
dhara_storage = "0.8.0"
```

```rust
use dhara_storage::{FileStorage, analyze_path};

let report = analyze_path("sample.pdf")?;
let bytes = FileStorage::from_existing("sample.pdf")?.read()?;
# Ok::<(), dhara_storage::StorageError>(())
```

**.NET** — install [Dhara.Storage][readme-nuget]:

```powershell
dotnet add package Dhara.Storage --version 0.8.0
```

**Operator** — verify package shape and dry-run release:

```powershell
cargo run -p dhara_tool -- verify package
cargo run -p dhara_tool -- release run --dry-run
```

**Troubleshooting**

- Missing native RID at runtime → ensure the NuGet package includes your `runtimes/{rid}/native` asset; see [CI/CD reference][ci-cd].
- Local `dotnet pack` blocked → use `dhara_tool` staging; single-runtime packs are intentionally guarded.
- Wrong worker count in defs builds → set `-w` / `--workers` or `TOOL_MAX_WORKERS`; see [logging reference][logging].

## ✅ Testing & Quality Assurance

```powershell
# Full local parity with CI (fmt, clippy, doc, Rust + .NET tests)
./tooling/scripts/verify-local.ps1

# Per-crate Rust tests
cargo test -p dhara_storage --all-features
cargo test -p dhara_storage_dal
cargo test -p dharastorage-ffi

# NuGet package verification (after native staging)
cargo run -p dhara_tool -- verify package
```

Skip `cargo doc` with `./tooling/scripts/verify-local.ps1 -SkipDocs` when iterating quickly.

## 🤝 Contributing & License

Open a pull request against `main`. Keep workspace and package READMEs accurate when behavior or publish surfaces change.

Licensed under [Apache-2.0][license]. See per-crate `Cargo.toml` and the NuGet package for attribution.

**Technical reference** (ABI, DSFD format, CI maps, logging): [docs index][docs-index].

[readme-dhara-storage]: src/core/dhara_storage/README.md
[readme-dal]: src/core/dhara_storage_dal/README.md
[readme-dharastorage]: src/bindings/dharastorage-ffi/README.md
[readme-nuget]: src/bindings/csharp/Dhara.Storage/README.md
[readme-tool]: tooling/dhara_tool/README.md
[verify-local]: tooling/scripts/verify-local.ps1
[env-example]: .env.example
[dhara-config]: dhara.config.toml
[ci-cd]: docs/ci-cd-pipelines.md
[logging]: docs/logging.md
[license]: LICENSE.txt
[docs-index]: docs/README.md
