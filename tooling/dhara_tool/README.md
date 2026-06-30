# dhara_tool

[![License: Apache-2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://github.com/D-Naveenz/dhara_storage/blob/main/LICENSE.txt)

`dhara_tool` is the operator CLI for the Dhara Storage workspace.
It syncs shared config, builds and verifies definition packages, stages native assets, validates NuGet shape, and runs release publishes.
For fmt/clippy/doc/tests parity with CI, prefer [verify-local][verify-local] over inventing one-off cargo invocations.

## ✨ Key Features

- **Config sync** — propagates [dhara.config.toml][dhara-config] into manifests
- **Definitions pipeline** — pack, build TrID XML, inspect, verify, sync embedded `filedefs.dat`
- **Native staging** — per-OS `runtimes/{rid}/native` trees for NuGet
- **Package verify** — checks merged native layout before publish
- **Release orchestration** — crates.io + NuGet publish with dry-run support
- **Interactive TUI** — launch without a subcommand in a real terminal

## 📦 Tech Stack & Architecture

| Piece | Role |
|-------|------|
| Clap | Subcommand parsing (direct mode) |
| Ratatui | Interactive operator TUI |
| Rayon | Parallel TrID parse/reduce |
| `dhara_storage_dal` | DSFD encode/decode for defs commands |

```
dhara_tool/src/
├── commands/        # config, defs, verify, package, release, version
├── tui/             # interactive mode
└── logging/         # audit log setup

tooling/
├── scripts/         # CI wrappers (stage-native, merge, verify-package)
├── output/          # NuGet packages and operator artifacts
├── logs/            # audit logs ({date}_dhara_tool*.log)
└── artifacts/       # gitignored native staging scratch
```

CI vs tool split: [CI/CD reference][ci-cd]. Audit log rules: [logging reference][logging].

## 🚀 Getting Started & Installation

**Prerequisites:** Rust stable. .NET 10 when running full [verify-local][verify-local].

From the workspace root:

```powershell
cargo run -p dhara_tool -- --help
```

Launch the TUI (interactive mode — no subcommand, real TTY):

```powershell
cargo run -p dhara_tool
```

## 🔧 Configuration & Environment Variables

Shared metadata: [dhara.config.toml][dhara-config] at the repo root.
Publish secrets: `.env.local` (from [.env.example][env-example]).

| Variable | Purpose |
|----------|---------|
| `CARGO_REGISTRY_TOKEN` | crates.io publish |
| `NUGET_API_KEY` | NuGet.org publish |
| `NUGET_SOURCE` | NuGet feed URL |
| `TOOL_MAX_WORKERS` | Caps Rayon workers (`-w` / `--workers` wins) |

`RAYON_NUM_THREADS` is **ignored** — use `-w` or `TOOL_MAX_WORKERS` instead.

Logging flags: default INFO on console and file; `-m` / `--min` for WARN-only file logs; `-t` / `--trace` for DEBUG file detail.

## 🛠️ Usage Examples

| Section | Commands |
|---------|----------|
| `config` | `show`, `sync`, `env init` |
| `version` | `set`, `bump` |
| `defs` | `pack`, `build-trid-xml`, `inspect`, `inspect-trid-xml`, `normalize`, `verify`, `sync-embedded` |
| `verify` | `package` |
| `package` | `pack`, `stage-native`, `publish` |
| `release` | `run` |

```powershell
./tooling/scripts/verify-local.ps1
cargo run -p dhara_tool -- config sync
cargo run -p dhara_tool -- defs sync-embedded
cargo run -p dhara_tool -- verify package
cargo run -p dhara_tool -- release run --dry-run
```

**Troubleshooting**

- Missing TrID input → place archives under [tooling/dhara_tool/package/][package-readme]; see [DSFD reference][filedefs-dat].
- CD publish missing artifacts → merge commit SHA must match PR CI artifacts; see [CI/CD reference][ci-cd].
- Sparse file logs → use `-t` / `--trace`; log path is DEBUG-only on session start.

## ✅ Testing & Quality Assurance

```powershell
cargo test -p dhara_tool
cargo clippy -p dhara_tool --all-targets -- -D warnings
```

Full workspace gate:

```powershell
./tooling/scripts/verify-local.ps1
```

Audit logs land in `tooling/logs/{date}_dhara_tool[_N].log`.

## 🤝 Contributing & License

Part of the [Dhara Storage workspace][repo-root]. Licensed under Apache-2.0.

[repo-root]: https://github.com/D-Naveenz/dhara_storage
[verify-local]: ../../scripts/verify-local.ps1
[dhara-config]: ../../dhara.config.toml
[env-example]: ../../.env.example
[ci-cd]: ../../docs/ci-cd-pipelines.md
[logging]: ../../docs/logging.md
[filedefs-dat]: ../../docs/filedefs-dat.md
[package-readme]: package/README.md
