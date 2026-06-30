# Dhara Storage — technical reference

This directory holds versioned technical reference for the Dhara Storage workspace.
Read these docs for ABI contracts, binary formats, CI/CD maps, and operator logging — depth that READMEs intentionally omit.

For onboarding, install steps, and package overviews, start at the [workspace README][root-readme] and per-package READMEs linked below.

## Documentation map

| Doc | Audience | Topic |
|-----|----------|-------|
| [Logging conventions][logging] | Operators, agents | `dhara_tool` audit tiers, session lifecycle, TrID phase lines |
| [filedefs.dat / DSFD format][filedefs-dat] | Implementers | Binary layout, metadata footer, defs build pipeline |
| [Typed C-compatible ABI][typed-abi] | FFI authors | `#[repr(C)]` rules, ownership, Rust ↔ C# marshalling |
| [CI/CD pipelines][ci-cd] | Release engineers | GitHub Actions jobs, native merge, `dhara_tool` touchpoints |
| [Multi-platform native packaging][native-packaging] | Release engineers, FFI authors | RID staging, merge/pack pitfalls, troubleshooting |

## Package READMEs (publish surfaces)

| README | Surface |
|--------|---------|
| [Workspace][root-readme] | GitHub repo landing |
| [dhara_storage][readme-dhara-storage] | crates.io |
| [dhara_storage_dal][readme-dal] | crates.io |
| [dharastorage][readme-dharastorage] | Native ABI (NuGet asset) |
| [Dhara.Storage][readme-nuget] | NuGet.org package readme |
| [dhara_tool][readme-tool] | Operator CLI (workspace) |

## Conventions

- **README vs docs** — READMEs onboard humans on each publish surface; `docs/` explains how things work in depth.
- **Evidence first** — treat manifests, workflows, and source as authoritative over stale prose.
- **MindVault** — private workspace memory and durable cross-repo lessons stay outside this repository.

## Related

- [AGENTS.md][agents] — agent router and local commands
- [dhara.config.toml][dhara-config] — shared version and publish metadata
- [pipeline workflow][pipeline-yml] — canonical CI/CD definition

[root-readme]: ../README.md
[readme-dhara-storage]: ../src/core/dhara_storage/README.md
[readme-dal]: ../src/core/dhara_storage_dal/README.md
[readme-dharastorage]: ../src/dharastorage/README.md
[readme-nuget]: ../src/bindings/Dhara.Storage/README.md
[readme-tool]: ../tooling/dhara_tool/README.md
[logging]: logging.md
[filedefs-dat]: filedefs-dat.md
[typed-abi]: typed-c-compatible-abi.md
[ci-cd]: ci-cd-pipelines.md
[native-packaging]: native-packaging.md
[agents]: ../AGENTS.md
[dhara-config]: ../dhara.config.toml
[pipeline-yml]: ../.github/workflows/pipeline.yml
