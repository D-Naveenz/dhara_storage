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
Dhara TUI (**interactive** mode). Explicit subcommands use **direct** mode (no TUI).

## Logging

Audit logs follow [docs/logging.md](../docs/logging.md). Each run writes to
`tooling/output/logs/{date}_dhara_tool[_N].log` with session-scoped files.

`dhara_tool` emits human-readable audit lines for:

- session and module lifecycle (start, steps, finish)
- TrID transformation statistics
- subprocess milestones (verify, package, release)
- failures with exit codes and timestamps

Use `-v` for more console detail; `-q` suppresses command stdout in direct mode.

Repository-specific command logic lives in `tooling/dhara_tool/src/ops`. The TUI and CLI registry in `command` and `tui` call into that module through `DharaStorageCapability`.

## Output layout

- `tooling/output/` — generated artifacts (`filedefs.dat`, NuGet packages, logs)
- `tooling/artifacts/` — gitignored staging for native staging, smoke builds, and local NuGet config during verification

The canonical runtime `filedefs.dat` lives at `tooling/output/filedefs.dat` and is embedded into `dhara_storage_dal` at compile time. Use `defs sync-embedded` to rebuild it from `tooling/dhara_tool/package/triddefs_xml.7z`.
