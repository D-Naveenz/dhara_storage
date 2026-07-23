# AGENTS.md

This workspace can use MindVault as optional local AI memory. Keep this file short: it is a router and quick reference, not the knowledge base.

## MindVault

- Use `$mindvault` / MindVault MCP to resolve the local vault and workspace evidence.
- Workspace identity is stored in `mindvault.toml` as `workspace_id`.
- If MindVault tools are unavailable, continue from repo files only.
- Store durable lessons and cross-workspace principles in MindVault, not in this repository.

## Purpose

- `src/core/dhara_storage` is the Rust-native core runtime for Dhara Storage.
- `src/bindings/dharastorage-ffi` is the C ABI layer for managed/native hosts.
- `src/bindings/csharp/Dhara.Storage` is the active .NET binding project.
- Submodule [`tooling/drot`](tooling/drot) ([dhara_repo_orchestration](https://github.com/D-Naveenz/dhara_repo_orchestration)) provides the operator CLI (`drot`) and TUI (`drot_tui`); `dhara.config.toml` is the product workspace config (not tool version).

## Local Commands

- Init submodule: `git submodule update --init --recursive`
- Ensure production-shaped CLI: `./tooling/scripts/ensure-drot-dist.ps1` (rebuilds when `target/dist/drot` version ≠ `tooling/drot/Cargo.toml`)
- Full local check (CI parity): `./tooling/scripts/verify-local.ps1` — ensures dist, then `target/dist/drot -r <repo> quality run`
- Windows GitHub SSH + LFS setup: `./tooling/scripts/setup-github-ssh.ps1` (analyze by default; `-Repair` or `-Recreate` to act)
- Active DROT development: work in the orchestration repo (or submodule); `cargo build --manifest-path tooling/drot/Cargo.toml -p drot`
- Verify NuGet package shape: `target/dist/drot -r . --yes verify package` (after ensure)
- **Tool version:** owned only by DROT (`tooling/drot/Cargo.toml`). Storage pins via submodule gitlink. **Workspace/NuGet manifest drift** is reconciled on the next `drot` run (confirm activation, or pass `--yes` in CI/scripts).

## CI/CD

- PR pipeline: [`.github/workflows/pipeline.yml`](.github/workflows/pipeline.yml) — see [docs/ci-cd-pipelines.md](docs/ci-cd-pipelines.md)
- Merge publishes: [`publish-crates.yml`](.github/workflows/publish-crates.yml), [`publish-nuget.yml`](.github/workflows/publish-nuget.yml) — path-filtered; `workflow_dispatch` when automation skips
- **PR quality** uses direct `cargo fmt/clippy/doc` (core + FFI only). **DROT** is downloaded as an artifact from `dhara_repo_orchestration` for the pinned submodule SHA ([`download-drot`](.github/actions/download-drot/action.yml)); requires secret `DROT_ARTIFACTS_TOKEN`.
- CD on merge reuses PR artifacts; use merge commits (not squash) so NuGet CD can resolve `HEAD^2`.
- **DROT ↔ DAL:** `drot_dhara_storage` pins published `dhara_storage_dal` from crates.io; optional `[patch.crates-io]` inside the DROT workspace for local co-dev.

## Local Guardrails

- Keep `dhara_storage` Rust-native; solve .NET interop constraints in `dharastorage-ffi` and `src/bindings/csharp/Dhara.Storage`.
- Treat Windows as the primary developer workstation; ship all five 64-bit RIDs via CI merge (`package stage-native` per OS + `native merge`).
- Repo code, manifests, tests, and workflow files win if a vault note drifts.
- Do not add local private paths or personal vault locations to this file.
