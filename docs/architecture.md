# Dhara Storage — workspace architecture

This document maps how the monorepo is organized after the tool-focused modularization pass: crate boundaries, operator-tool layering, bindings layout, publish pipelines, and how the tool couples to `dhara_storage_core` at compile time versus at release time.

## Repository layout

```mermaid
flowchart TB
  subgraph core [src/core]
    coreCrate[dhara_storage_core]
    runtime[dhara_storage]
  end

  subgraph bindings [src/bindings]
    ffi[dharastorage-ffi cdylib dharastorage]
    csharp[csharp Dhara.Storage]
  end

  subgraph tool [tooling/drot]
    kernel[drot_kernel]
    plugin[drot_dhara_storage]
    tui[drot_tui]
    bin[drot binary]
  end

  coreCrate --> runtime
  runtime --> ffi
  ffi --> csharp
  plugin --> kernel
  bin --> kernel
  bin --> plugin
  tui --> kernel
  tui --> plugin
  plugin -.->|path or registry pin| coreCrate
```

| Path | Role |
|------|------|
| `src/core/dhara_storage_core` | Framework / abstraction layer (`definitions` = DSFD today; no embedded defs) |
| `src/core/dhara_storage` | Business runtime; embeds `filedefs.dat` |
| `src/bindings/dharastorage-ffi` | C ABI crate (`dharastorage-ffi` package, `dharastorage` lib name) |
| `src/bindings/csharp/` | `Dhara.Storage` NuGet source, tests, consumer smoke |
| `tooling/drot/src/*` | Nested workspace: kernel → plugin → hosts (tui / binary) |
| `dhara.config.toml` | Workspace semver, tool semver, NuGet/CI metadata |

## Operator tool crates

The operator surface is a **nested Cargo workspace** under `tooling/drot/`. Version authority for the tool is `[workspace.package].version` in `tooling/drot/Cargo.toml`, owned by DROT; storage pins a submodule gitlink only.

Architecture: **drot_kernel** (framework) ← **drot_dhara_storage** (product plugin) ← **drot** / **drot_tui** (hosts).

```mermaid
flowchart LR
  subgraph kernel [drot_kernel]
    paths[paths]
    config[repo_config / activation]
    logging[logging]
    runtime_mod[subprocess / workers]
    host_apis[command / forms / runner / interactive]
    ctx[ToolContext / CommandResult]
  end

  subgraph plugin [drot_dhara_storage]
    commands[commands / registry]
    quality[ops: quality]
    verify[ops: verify]
    release[ops: release]
    nuget[ops: nuget]
    native[ops: native_merge / native_rids]
    defs[filedefs]
  end

  subgraph tui [drot_tui]
    screens[screens / widgets]
    app[app.rs event loop]
  end

  plugin --> kernel
  tui --> plugin
  tui --> kernel
  bin[drot bin] --> plugin
  bin --> kernel
```

Hosts (`drot`, `drot_tui`) depend on both kernel and plugin; plugin depends on kernel.

### Commands vs operations

| Layer | Responsibility | Example |
|-------|----------------|---------|
| **Hosts** (`drot` / `drot_tui`) | Binary orchestration and TUI event loop | argv → dispatch; screens / widgets |
| **Plugin** (`drot_dhara_storage`) | Product commands, registry, domain ops, filedefs | `quality::run_clippy`, `release::run_cargo_release` |
| **Kernel** (`drot_kernel`) | Host APIs (command, forms, runner, interactive), paths, config activation, logging, subprocess helpers, weighted progress | `detect_config_drift`, `operation_progress` |

**Version bump example:** `version bump patch` in the host calls `repo_config::bump_workspace_version` in kernel, which writes `dhara.config.toml` and (on activation) syncs root `Cargo.toml` workspace deps. Tool version bumps happen only in the DROT repo Cargo.toml.

`app.rs` lives in the **binary crate** because it orchestrates CLI and TUI; neither the binary host wiring nor `drot_tui` can depend on each other without a cycle.

### TUI layout (`drot_tui`)

| Region | Role |
|--------|------|
| **Tasks tree** | Favorites + command hierarchy from `drot_kernel::interactive::tree` |
| **Tabs** | Info, Options, Troubleshooting (warn/error only), System configs (read-only) |
| **Action panel** | Unit-sum progress bar with % overlay, stage status line, Run/Cancel/Reset |
| **Chrome** | Title bar (version + repo), bottom command shortcut bar |

Progress is driven by `drot_kernel::operation_progress` (analyze → discover totals → commit plan → tick stages; unit-sum %). Workflows in `drot_dhara_storage::ops` call progress hooks at subprocess boundaries — not stdout parsing. See [TUI operation progress](tui-progress.md). `drot_kernel::runner` installs `OperationProgressGuard` for interactive runs (no placeholder single-step plan).

`drot_dhara_storage::registry` is split by command section (`config`, `defs`, `quality`, `package`, `release`) with shared `ui` metadata helpers.

## Path resolution and config

`drot` separates **exe_path** (directory of the running binary) from **repo_path** (directory containing `dhara.config.toml`).

| Anchor | Resolution | Outputs |
|--------|------------|---------|
| `exe_path` | `resolve_exe_root(current_exe)` | `logs/`, `output/`, `artifacts/`, `runtime.toml` |
| `repo_path` | `-r` / `--repository`, then `runtime.toml`, then prompt or GUI picker | `filedefs.dat`, Cargo/dotnet sources, config activation |

`is_repo_root` requires only `dhara.config.toml`. `-r` accepts a repository directory or a direct path to that file.

`ToolContext` carries `repo_root`, `tool_root` (`exe_path`), activated `DharaRepoConfig`, and logging handles. CLI and GUI construct it once per invocation after repository resolution; ops functions take `&ToolContext` or explicit paths derived from it.

## Publish pipelines (ops)

Release logic in `drot_dhara_storage::ops::release` splits cleanly for CI:

| Workflow | Ops entry | Flags |
|----------|-----------|-------|
| `publish-crates.yml` | `release run` | `--skip-nuget` |
| `publish-nuget.yml` | `release run` | `--skip-cargo --prepacked-nuget <path>` |

PR CI (`pipeline.yml`) still produces `release-native-stage` and `release-nuget-package` artifacts; merge publishes download them at `HEAD^2` (merge second parent).

## Tool ↔ core coupling

| Concern | Mechanism |
|---------|-----------|
| **Compile-time codec (interim)** | `drot_dhara_storage` pins published `dhara_storage_dal` from crates.io so orchestration CI builds without a local core tree |
| **Compile-time core (after publish)** | Switch plugin dep to `dhara_storage_core = "0.9.6"`; optional DROT workspace `[patch.crates-io]` to the local path |
| **Embedded defs bytes** | `defs sync-embedded` writes git-tracked `src/core/dhara_storage/resources/filedefs.dat` — data path, not a path dependency |
| **Package version read** | Builder stamps `packageVersion` from linked codec crate `PACKAGE_VERSION` (dal interim; core after cutover) |

| Artifact | Version authority | Typical bump |
|----------|-------------------|--------------|
| `dhara_storage` / `dhara_storage_core` | `[versions].workspace` in `dhara.config.toml` | Minor release |
| `drot` | `tooling/drot/Cargo.toml` only | Independent tool releases via submodule pin |
| Tool codec dep | crates.io `dhara_storage_dal` (interim) → `dhara_storage_core` after publish | Patch when publishing hotfix codec |

CI `pipeline.yml` may download DROT artifacts or build from source; the interim dal pin keeps orchestration artifact builds green.
## Related docs

- [CI/CD pipelines][ci-cd] — four-workflow map and path filters
- [filedefs.dat format][filedefs-dat]
- [Typed C-compatible ABI][typed-abi]
- [Native packaging][native-packaging]

[ci-cd]: ci-cd-pipelines.md
[filedefs-dat]: filedefs-dat.md
[typed-abi]: typed-c-compatible-abi.md
[native-packaging]: native-packaging.md
