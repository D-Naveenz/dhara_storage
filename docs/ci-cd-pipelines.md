# CI/CD Pipeline Flow

Human-readable map of GitHub Actions workflows and where `dhara_tool` is used versus direct CLI commands.

## Triggers

| Workflow | Event | Jobs |
|----------|-------|------|
| [pipeline.yml][pipeline-yml] | `pull_request` | `code quality (linux)`, `platform (*)`, `NuGet package (linux)`, `NuGet verify (windows)` |
| [pipeline.yml][pipeline-yml] | `workflow_dispatch` (`force_tool_rebuild`) | Same jobs |
| [publish-crates.yml][publish-crates-yml] | `push` to `main` (cargo scope) | `detect-changes`, `cargo release (linux)` |
| [publish-crates.yml][publish-crates-yml] | `workflow_dispatch` | `detect-changes`, `cargo release (linux)` |
| [publish-nuget.yml][publish-nuget-yml] | `push` to `main` (nuget scope) | `detect-changes`, `nuget release (linux)` |
| [publish-nuget.yml][publish-nuget-yml] | `workflow_dispatch` | `detect-changes`, `nuget release (linux)` |

**Concurrency:** PR pipeline runs cancel in-progress; merge publishes do not.

## Architecture

```mermaid
flowchart TB
  subgraph pr ["pipeline.yml pull_request"]
    Q["code quality (linux)"]
    PW["platform (windows)"]
    PL["platform (linux)"]
    PLA["platform (linux arm64)"]
    PM["platform (macos)"]
    PACK["NuGet package (linux)"]
    VER["NuGet verify (windows)"]
    Q --> PW & PL & PLA & PM
    PW & PL & PLA & PM --> PACK
    PACK --> VER
    PACK --> ART[release artifacts]
  end

  subgraph cd_cargo ["publish-crates.yml"]
    FC[cargo_scope filter]
    CR["cargo release (linux)"]
    FC --> CR
  end

  subgraph cd_nuget ["publish-nuget.yml"]
    FN[nuget_scope filter]
    NU["nuget release (linux)"]
    ART --> NU
    FN --> NU
  end
```

## Tool vs direct commands

| Work | CI implementation |
|------|-------------------|
| `fmt` / `clippy` / `doc` (core + FFI) | Direct `cargo` on `ubuntu-latest` — **no tool**; [`setup-linux-tool-deps`](../.github/actions/setup-linux-tool-deps/action.yml) for GTK/glib (`dhara_storage` / `file_icon_provider`) |
| `fmt` on `dhara_tool` | Direct `cargo fmt` only (no clippy/doc for tool in CI) |
| `cargo test` (core crates) | Direct `cargo test` in each `platform (*)` job |
| `dotnet test` | Direct `dotnet test` on `platform (windows)` only |
| Native staging (Linux/macOS) | Direct `cargo build -p dharastorage-ffi --release --target …` + copy into `runtimes/` |
| Native staging (Windows) | `dhara_tool package stage-native --msvc-env` (MSVC re-exec stays in tool) |
| `dhara_tool` dist build | `cargo build -p dhara_tool --profile dist` on cache miss per OS |
| Native merge | Inline shell copy of `runtimes/` trees (no tool) |
| `package pack` | `dhara_tool package pack` on `NuGet package (linux)` with merged `--native-stage` |
| `verify package` | `dhara_tool verify package` on `NuGet verify (windows)` — smoke/AOT require `win-x64` host |
| Cargo CD | Direct `cargo release …` ([`publish-crates.yml`](../.github/workflows/publish-crates.yml)) |
| NuGet CD | Direct `dotnet nuget push` ([`publish-nuget.yml`](../.github/workflows/publish-nuget.yml)) |

**Linux GUI rule:** on Linux jobs that **link** `dhara_storage` (clippy/tests with default deps) or **build** `dhara_tool`, run [`setup-linux-tool-deps`](../.github/actions/setup-linux-tool-deps/action.yml) first (glib, gtk, pkg-config, wayland). Restoring a cached dist binary alone does not require those packages.

## Tool cache

- **Cache key:** `dhara-tool-{source-hash}-{os-arch}` where `source-hash` is a SHA256 prefix over tracked `tooling/dhara_tool/**`, root `Cargo.toml`, and `Cargo.lock` (computed inline in workflow bash).
- **Warmers:** each `platform (*)` job restores or builds and saves its OS cache entry; `NuGet package (linux)` reuses `linux-x64`.
- **Version metadata:** `[tool].version` in [`dhara.config.toml`](../dhara.config.toml) is for operator releases and local `ensure-dhara-tool-dist`; CI cache does not key on version.
- **Binary path:** `target/dist/dhara_tool` (`.exe` on Windows), `[profile.dist]` in root [`Cargo.toml`](../Cargo.toml).

## Path-scoped merge publishes

`dorny/paths-filter@v3` gates whether publish jobs run on `push` to `main`. `workflow_dispatch` always runs.

| Filter | Paths (illustrative) | Skips when merge only touches |
|--------|----------------------|-------------------------------|
| **cargo_scope** | `src/core/dhara_storage/**`, `src/core/dhara_storage_dal/**`, `dhara.config.toml`, root manifests | `tooling/**`, `docs/**`, bindings-only |
| **nuget_scope** | `src/core/**`, `src/bindings/**`, `dhara.config.toml`, root manifests | `tooling/**`, `docs/**`, pure markdown |

NuGet CD still **requires PR artifacts** from `NuGet package (linux)` at merge second parent (`HEAD^2`).

## PR jobs

### `code quality (linux)`

Direct commands (no `dhara_tool`); [`setup-linux-tool-deps`](../.github/actions/setup-linux-tool-deps/action.yml) for GTK/glib:

- `cargo fmt -p dhara_storage_dal -p dhara_storage -p dharastorage-ffi -p dhara_tool --check`
- `cargo clippy` on `dhara_storage` (all targets/features), then `dhara_storage_dal` + `dharastorage-ffi`
- `cargo doc --no-deps` on core + FFI only

### `platform (windows|linux|linux arm64|macos)`

After `code quality (linux)`:

1. [`setup-linux-tool-deps`](../.github/actions/setup-linux-tool-deps/action.yml) on Linux (always — tests link `dhara_storage`).
2. Compute tool source hash; restore `target/dist` from Actions cache.
3. On cache miss: build `dhara_tool`; save cache.
4. Direct Rust tests; `dotnet test` on Windows only.
5. Stage native assets (tool on Windows with `--msvc-env`; direct `cargo build` elsewhere).
6. Upload `native-stage-{windows,linux,linux-arm64,macos}`.

CI does **not** run `cargo test -p dhara_tool`; developers validate the tool locally.

### `NuGet package (linux)`

After all platform jobs:

1. Restore or build `linux-x64` `dhara_tool` (with Linux GUI deps on build).
2. Download four native-stage artifacts; merge `runtimes/` inline.
3. `dhara_tool package pack --native-stage target/dist/artifacts/native-stage`
4. Upload `release-native-stage`, `release-nuget-package`, `release-metadata` (90-day retention).

### `NuGet verify (windows)`

After `NuGet package (linux)`:

1. Restore or build `windows-x64` `dhara_tool`.
2. Download `release-native-stage` artifact.
3. `dhara_tool verify package --native-stage target/dist/artifacts/native-stage` (ConsumerSmoke + AOT on `win-x64`).

## CD: `publish-crates`

1. `detect-changes` — `cargo_scope` filter (or always on `workflow_dispatch`).
2. [`setup-linux-tool-deps`](../.github/actions/setup-linux-tool-deps/action.yml) — `cargo release` verifies crate tarballs by building `dhara_storage` (GTK/glib via `file_icon_provider`).
3. `cargo release --workspace --isolated --allow-branch main --tag-name 'v{{version}}' --no-confirm --execute` (`CARGO_REGISTRY_TOKEN`). Dry-run uses `--allow-branch '*'` and `--no-verify`.

## CD: `publish-nuget`

1. `detect-changes` — `nuget_scope` filter (or always on `workflow_dispatch`).
2. Resolve artifact commit (`HEAD^2` for merge commits) — see [native packaging][native-packaging].
3. Download PR CI artifacts for that commit.
4. `dotnet nuget push` with `NUGET_API_KEY` and source from `dhara.config.toml`.

## Local parity

[`ensure-dhara-tool-dist.ps1`][ensure-dist-ps1] / [`.sh`][ensure-dist-sh] version-gate the dist binary. [`verify-local.ps1`][verify-local-ps1] runs the full tool quality surface (including tool clippy/tests) — stricter than PR CI.

## Related docs

- [Workspace architecture][architecture] — tool crate DAG
- [Multi-platform native packaging][native-packaging] — RID staging, artifact SHA pitfalls
- [Logging conventions][logging] — audit logs under `{tool_root}/logs/`
- [dhara_tool README][readme-tool] — full command surface
- [Docs index][docs-index]

[pipeline-yml]: ../.github/workflows/pipeline.yml
[publish-crates-yml]: ../.github/workflows/publish-crates.yml
[publish-nuget-yml]: ../.github/workflows/publish-nuget.yml
[workspace-cargo]: ../Cargo.toml
[verify-local-ps1]: ../tooling/scripts/verify-local.ps1
[verify-local-sh]: ../tooling/scripts/verify-local.sh
[ensure-dist-ps1]: ../tooling/scripts/ensure-dhara-tool-dist.ps1
[ensure-dist-sh]: ../tooling/scripts/ensure-dhara-tool-dist.sh
[logging]: logging.md
[native-packaging]: native-packaging.md
[architecture]: architecture.md
[readme-tool]: ../tooling/dhara_tool/README.md
[docs-index]: README.md
