# dhara_tool

`dhara_tool` is the supported operator CLI for this repository.

It acts as the front door for:

- repo config and version synchronization
- local CI-style verification
- multi-runtime NuGet packaging and publish flows
- definitions package workflows
- interactive TUI usage for common maintenance paths

## Examples

```powershell
cargo run -p dhara_tool -- verify ci
cargo run -p dhara_tool -- verify package
cargo run -p dhara_tool -- release run --dry-run
cargo run -p dhara_tool -- release run --skip-cargo
```

Launching `dhara_tool` without a subcommand in an interactive terminal opens the
Dhara TUI. Explicit subcommands still use the minimal non-TUI execution path.

## Logging

`dhara_tool` now emits richer structured logs for:

- command start and completion
- effective configuration
- spawned external processes
- package verification and publish milestones
- failures and validation details

Repository-specific command logic lives in `tooling/dhara_tool/src/ops`. The TUI and CLI registry in `command` and `tui` call into that module through `DharaStorageCapability`.

## Output layout

- `tooling/output/` — generated artifacts (`filedefs.dat`, NuGet packages, logs)
- `tooling/artifacts/` — gitignored staging for native staging, smoke builds, and local NuGet config during verification

The canonical runtime `filedefs.dat` lives at `tooling/output/filedefs.dat` and is embedded into `dhara_storage_dal` at compile time. Use `defs sync-embedded` to rebuild it from `tooling/dhara_tool/package/triddefs_xml.7z`.
