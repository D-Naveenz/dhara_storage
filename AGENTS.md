# AGENTS.md

This workspace can use MindVault as optional local AI memory. Keep this file short: it is a router and quick reference, not the knowledge base.

## MindVault

- Use `$mindvault` / MindVault MCP to resolve the local vault and workspace evidence.
- Workspace identity is stored in `mindvault.toml` as `workspace_id`.
- If MindVault tools are unavailable, continue from repo files only.
- Store durable lessons and cross-workspace principles in MindVault, not in this repository.

## Purpose

- `src/static/dhara_storage` is the Rust-native core runtime for Dhara Storage.
- `src/dynamic/dharastorage` is the C ABI layer for managed/native hosts.
- `src/bindings/Dhara.Storage` is the active .NET binding project.
- `dhara_tool` and `dhara.config.toml` are the supported operator surface for config sync, verification, packaging, and publishing flows.

## Local Commands

- Verify CI-equivalent checks: `cargo run -p dhara_tool -- verify ci`
- Verify NuGet package shape: `cargo run -p dhara_tool -- verify package`
- Sync shared config into manifests: `cargo run -p dhara_tool -- config sync`

## Local Guardrails

- Keep `dhara_storage` Rust-native; solve .NET interop constraints in `dharastorage` and `src/bindings/Dhara.Storage`.
- Treat Windows as the primary runtime and CI target unless a concrete portability goal says otherwise.
- Repo code, manifests, tests, and workflow files win if a vault note drifts.
- Do not add local private paths or personal vault locations to this file.
