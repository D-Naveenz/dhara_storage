# AGENTS.md

This workspace can use MindVault as optional local AI memory. Keep this file short: it is a router and quick reference, not the knowledge base.

## MindVault

- Use `$mindvault` / MindVault MCP to resolve the local vault and workspace evidence.
- Workspace identity is stored in `mindvault.toml` as `workspace_id`.
- If MindVault tools are unavailable, continue from repo files only.
- Store durable lessons and cross-workspace principles in MindVault, not in this repository.

## Purpose

- `src/core/dhara_storage` is the Rust-native core runtime for Dhara Storage.
- `src/dharastorage` is the C ABI layer for managed/native hosts.
- `src/bindings/Dhara.Storage` is the active .NET binding project.
- `dhara_tool` and `dhara.config.toml` are the supported operator surface for configuration, verification, packaging, and publishing flows.

## Local Commands

- Ensure production-shaped tool binary: `./tooling/scripts/ensure-dhara-tool-dist.ps1` (rebuilds only when `target/dist/` version ≠ `tooling/dhara_tool/Cargo.toml`)
- Full local check (CI parity): `./tooling/scripts/verify-local.ps1` — ensures dist, then `target/dist/dhara_tool quality run`
- Active tool development: `cargo run -p dhara_tool` / `cargo test -p dhara_tool` (dev profile; does not replace dist until version bump + ensure)
- Verify NuGet package shape: `target/dist/dhara_tool --yes verify package` (after ensure) or `cargo run -p dhara_tool -- --yes verify package`
- **Tool version bump:** update `[tool].version` in `dhara.config.toml` and `package.version` in `tooling/dhara_tool/Cargo.toml` in the same commit. **Workspace/NuGet manifest drift** is reconciled on the next tool run (confirm activation, or pass `--yes` in CI/scripts).

## CI/CD

- PR/release pipeline: [`.github/workflows/pipeline.yml`](.github/workflows/pipeline.yml)
- Tool cache build: [`.github/workflows/dhara-tool-build.yml`](.github/workflows/dhara-tool-build.yml) — see [docs/ci-cd-pipelines.md](docs/ci-cd-pipelines.md)
- Pipeline jobs restore cached `dhara_tool` (`target/dist/`) by `[tool].version`; they do not compile the tool per job.
- **Linux-primary orchestration:** `quality`, `publish-readiness`, and CD `publish` use `ubuntu-latest` + `linux-x64` tool cache; only `platform-windows` stays on Windows for MSVC `stage-native`.
- **Tool version bump required** for any `tooling/dhara_tool/**` change (cache is version-keyed).
- CD on merge reuses PR artifacts (`--prepacked-nuget`); use merge commits (not squash) so CD can resolve the PR branch tip (`HEAD^2`).

## Local Guardrails

- Keep `dhara_storage` Rust-native; solve .NET interop constraints in `dharastorage` and `src/bindings/Dhara.Storage`.
- Treat Windows as the primary developer workstation; ship all five 64-bit RIDs via CI merge (`package stage-native` per OS + `native merge`).
- Repo code, manifests, tests, and workflow files win if a vault note drifts.
- Do not add local private paths or personal vault locations to this file.
