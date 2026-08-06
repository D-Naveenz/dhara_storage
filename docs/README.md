# Dhara Storage — technical reference

This directory holds versioned technical reference for the **Dhara Storage** product.
Read these docs for ABI contracts, binary formats, CI/CD maps, and native packaging — depth that READMEs intentionally omit.

**Operator tool (DROT)** docs live in the submodule: [`tooling/drot/docs/`](../tooling/drot/docs/README.md) and [`tooling/drot/AGENTS.md`](../tooling/drot/AGENTS.md). Stubs below redirect for old links.

For onboarding, install steps, and package overviews, start at the [workspace README][root-readme] and per-package READMEs linked below.

## Documentation map

| Doc | Audience | Topic |
|-----|----------|-------|
| [filedefs.dat / DSFD format][filedefs-dat] | Implementers | Binary layout, metadata footer, defs build pipeline |
| [Typed C-compatible ABI][typed-abi] | FFI authors | `#[repr(C)]` rules, ownership, Rust ↔ C# marshalling |
| [CI/CD pipelines][ci-cd] | Release engineers | GitHub Actions jobs, native merge, `drot` host touchpoints |
| [Workspace architecture][architecture] | Agents, contributors | Storage layout, bindings, publish split; DROT pointer |
| [Daemon transport][daemon-transport] | Implementers | `dhara-sd` named pipes / UDS plan, handle transfer |
| [Binding benchmarks][binding-benchmarks] | Implementers | FFI vs daemon ladder, BenchmarkDotNet harness |
| [Windows code signing][windows-signing] | Release engineers | winresource vs Authenticode / SAC |
| [Multi-platform native packaging][native-packaging] | Release engineers, FFI authors | RID staging, merge/pack pitfalls, troubleshooting |
| [Logging (redirect)][logging] | Operators, agents | → DROT `docs/logging.md` |
| [TUI progress (redirect)][tui-progress] | Operators, agents | → DROT `docs/tui-progress.md` |

## Package READMEs (publish surfaces)

| README | Surface |
|--------|---------|
| [Workspace][root-readme] | GitHub repo landing |
| [dhara_storage][readme-dhara-storage] | crates.io |
| [dhara_storage_core][readme-core] | crates.io |
| [dharastorage][readme-dharastorage] | Native ABI (NuGet asset) |
| [Dhara.Storage][readme-nuget] | NuGet.org — primary package |
| [Dhara.Storage.Extensions.Hosting][readme-hosting] | NuGet.org — Generic Host extension |
| [drot][readme-tool] | Operator CLI/TUI (submodule) |

## Conventions

- **README vs docs** — READMEs onboard humans on each publish surface; `docs/` explains how things work in depth.
- **DROT ownership** — TUI, operator logging, and tool crate architecture are maintained under `tooling/drot/`, not duplicated here.
- **Evidence first** — treat manifests, workflows, and source as authoritative over stale prose.
- **MindVault** — private workspace memory and durable cross-repo lessons stay outside this repository.

## Related

- [AGENTS.md][agents] — storage agent router and local commands
- [DROT AGENTS.md][drot-agents] — tool agent router
- [dhara.config.toml][dhara-config] — shared version, `[product]`, NuGet feed, `[ci]` pack paths
- [DROT host config][drot-host-config] — config vs csproj ownership, secrets, activation
- [pipeline workflow][pipeline-yml] — canonical CI/CD definition

[root-readme]: ../README.md
[readme-dhara-storage]: ../core/dhara_storage/README.md
[readme-core]: ../core/dhara_storage_core/README.md
[readme-dharastorage]: ../interop/dharastorage-ffi/README.md
[readme-nuget]: ../bindings/csharp/Dhara.Storage/README.md
[readme-hosting]: ../bindings/csharp/Dhara.Storage.Extensions.Hosting/README.md
[readme-tool]: ../tooling/drot/README.md
[logging]: logging.md
[tui-progress]: tui-progress.md
[filedefs-dat]: filedefs-dat.md
[typed-abi]: typed-c-compatible-abi.md
[ci-cd]: ci-cd-pipelines.md
[architecture]: architecture.md
[daemon-transport]: daemon-transport.md
[binding-benchmarks]: binding-benchmarks.md
[windows-signing]: windows-code-signing.md
[native-packaging]: native-packaging.md
[agents]: ../AGENTS.md
[drot-agents]: ../tooling/drot/AGENTS.md
[dhara-config]: ../dhara.config.toml
[drot-host-config]: ../tooling/drot/docs/host-config.md
[pipeline-yml]: ../.github/workflows/pipeline.yml
