# dhara_tool

[![License: Apache-2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://github.com/D-Naveenz/dhara_storage/blob/main/LICENSE.txt)

`dhara_tool` is the operator CLI for the Dhara Storage workspace.
It syncs shared config, builds and verifies definition packages, stages native assets, validates NuGet shape, and runs release publishes.
For fmt/clippy/doc/tests parity with CI, prefer [verify-local][verify-local] over inventing one-off cargo invocations.

## ✨ Key Features

- **Config activation** — `dhara.config.toml` is truth; startup prompts reconcile manifests (or `--yes` in CI). Tool bumps: update `[tool].version` and tool `Cargo.toml` together.
- **Definitions pipeline** — pack, build TrID XML, inspect, verify, sync embedded `filedefs.dat`
- **Quality gates** — `quality fmt`, `clippy`, `doc`, `test-rust`, `test-dotnet`, `run`
- **Native merge** — combine per-OS `runtimes/**` trees before pack
- **Package verify** — checks merged native layout before publish
- **Release orchestration** — crates.io + NuGet publish with dry-run support
- **Interactive GUI** — launch without a subcommand when a graphical display is available

## 📦 Tech Stack & Architecture

| Piece | Role |
|-------|------|
| Clap | Subcommand parsing (direct mode) |
| iced | Interactive operator GUI |
| Rayon | Parallel TrID parse/reduce |
| `dhara_storage_dal` | DSFD encode/decode for defs commands |

```
tooling/dhara_tool/
├── crates/
│   ├── dhara_tool_kernel/   # paths, config, logging, defs I/O
│   ├── dhara_tool_ops/      # quality, verify, release, native merge
│   ├── dhara_tool_cli/      # registry, commands, forms, runner
│   ├── dhara_tool_gui/      # iced widgets, screens, app orchestration
│   └── dhara_tool/          # binary entry (CLI + GUI boot)
└── assets/                  # GUI chrome (e.g. chevron SVG)

{exe_path}/              # directory containing the running binary
├── logs/                # audit logs ({date}_dhara_tool*.log)
├── output/              # NuGet packages and operator artifacts
├── artifacts/           # native staging scratch (e.g. native-stage/)
└── runtime.toml         # cached repository path
```

With the dist binary (`target/dist/dhara_tool`), `exe_path` is `target/dist/`. `cargo run` uses `target/debug/` instead. Workspace sources (TrID inputs, embedded defs) stay under the repository — see [logging reference][logging].

**Repository resolution:** `-r` / `--repository` overrides `{exe_path}/runtime.toml`; otherwise cache, then CLI prompt or GUI repository picker on first launch.

CI vs tool split: [CI/CD reference][ci-cd]. Audit log rules: [logging reference][logging].

## 🚀 Getting Started & Installation

**Prerequisites:** Rust stable. .NET 10 when running full [verify-local][verify-local].

From the workspace root:

```powershell
cargo run -p dhara_tool -- --help
```

Launch the GUI (interactive mode — no subcommand, graphical display available):

```powershell
cargo run -p dhara_tool
```

Without a display (CI pipe, headless SSH), the same command prints plain-text help.

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
| `config` | `show`, `env init` |
| `version` | `set`, `bump` |
| `defs` | `pack`, `build-trid-xml`, `inspect`, `inspect-trid-xml`, `normalize`, `verify`, `sync-embedded` |
| `quality` | `fmt`, `clippy`, `doc`, `test-rust`, `test-dotnet`, `run` |
| `native` | `merge` |
| `verify` | `package` |
| `package` | `pack`, `stage-native` (`--msvc-env` on Windows), `publish` |
| `release` | `run` |

**Tool versioning:** bump `[tool].version` in [dhara.config.toml][dhara-config] and `[workspace.package].version` in [tooling/dhara_tool/Cargo.toml](Cargo.toml) together for tool-only changes (member crates inherit via `version.workspace = true`). After `version bump`, the next run offers to sync root `Cargo.toml` and the NuGet csproj from config. CI uses `--yes` to apply drift without prompting.

**Dist vs dev:** production-shaped binary lives at `target/dist/dhara_tool` (`[profile.dist]`). [`ensure-dhara-tool-dist`][ensure-dist-ps1] rebuilds only when the binary is missing or `--version` ≠ manifest. Use `cargo run -p dhara_tool` for day-to-day tool edits without invalidating dist.

```powershell
./tooling/scripts/ensure-dhara-tool-dist.ps1
./tooling/scripts/verify-local.ps1
./target/dist/dhara_tool -r . --yes config show
./target/dist/dhara_tool -r . --yes package stage-native --msvc-env
./target/dist/dhara_tool -r . --yes native merge --output target/dist/artifacts/native-stage --input ...
./target/dist/dhara_tool -r . --yes verify package
./target/dist/dhara_tool -r . --yes release run --dry-run
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

CI runs `cargo test -p dhara_tool` once on Linux in [dhara-tool-build][tool-build-yml]; matrix legs only compile `profile.dist` per OS. Platform-specific paths (MSVC re-exec, native merge) are exercised by [pipeline][ci-cd] jobs.

**VS Code:** tasks under `dhara-tool:` — `ensure dist`, `watch dev` (`cargo watch`, dev profile), `quality run (dist)`. Launch **Debug dhara_tool (dev)** for `cargo run`; **Run dhara_tool (dist)** ensures dist first. Requires [CodeLLDB][codelldb]; `cargo-watch` for the watch task.

Full workspace gate:

```powershell
./tooling/scripts/verify-local.ps1
```

Active tool iteration (does not rebuild dist):

```powershell
cargo test -p dhara_tool
cargo run -p dhara_tool --
```

Audit logs land in `{tool_root}/logs/{date}_dhara_tool[_N].log` (e.g. `target/dist/logs/` after `ensure-dhara-tool-dist`).

## 🤝 Contributing & License

Part of the [Dhara Storage workspace][repo-root]. Licensed under Apache-2.0.

[repo-root]: https://github.com/D-Naveenz/dhara_storage
[verify-local]: ../../scripts/verify-local.ps1
[dhara-config]: ../../dhara.config.toml
[env-example]: ../../.env.example
[ci-cd]: ../../docs/ci-cd-pipelines.md
[tool-build-yml]: ../../.github/workflows/dhara-tool-build.yml
[ensure-dist-ps1]: ../../scripts/ensure-dhara-tool-dist.ps1
[codelldb]: https://marketplace.visualstudio.com/items?itemName=vadimcn.vscode-lldb
[logging]: ../../docs/logging.md
[filedefs-dat]: ../../docs/filedefs-dat.md
[package-readme]: package/README.md
