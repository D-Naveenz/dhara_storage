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

Default logging is informative on the console and in the file log. Use `--minimal` to quiet the console, `--trace` for full reduce audit detail, and `-q` to suppress command stdout in direct mode.

Parallel TrID parse/reduce uses Rayon with a capped global thread pool. Cap workers with `-w` / `--workers` (default 4) or `TOOL_MAX_WORKERS`; `RAYON_NUM_THREADS` is ignored.

Source is organized by purpose under `tooling/dhara_tool/src/` (`filedefs/`, `logging/`, `registry.rs`, `commands.rs`, etc.). The TUI and CLI registry call domain modules through `DharaStorageCapability`.

## Output layout

- `tooling/output/` — generated artifacts (`filedefs.dat`, NuGet packages, logs)
- `tooling/artifacts/` — gitignored staging for native staging, smoke builds, and local NuGet config during verification

The canonical runtime `filedefs.dat` lives at `tooling/output/filedefs.dat` and is embedded into `dhara_storage_dal` at compile time. Use `defs sync-embedded` to rebuild it from `tooling/dhara_tool/package/triddefs_xml.7z`. See [docs/filedefs-dat.md](../docs/filedefs-dat.md) for the DSFD on-disk format.
