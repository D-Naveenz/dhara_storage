# dhara_tool

`dhara_tool` is the operator CLI for config sync, definitions packages, native staging, NuGet verification, and release flows.

For local CI parity (fmt, clippy, doc, tests), use [`tooling/scripts/verify-local.ps1`](../scripts/verify-local.ps1) or [`.sh`](../scripts/verify-local.sh) instead of a tool subcommand.

## Commands

| Section | Commands |
|---------|----------|
| `config` | `show`, `sync`, `env init` |
| `version` | `set`, `bump` |
| `defs` | `pack`, `build-trid-xml`, `inspect`, `inspect-trid-xml`, `normalize`, `verify`, `sync-embedded` |
| `verify` | `package` |
| `package` | `pack`, `stage-native`, `publish` |
| `release` | `run` |

## Examples

```powershell
./tooling/scripts/verify-local.ps1
cargo run -p dhara_tool -- verify package
cargo run -p dhara_tool -- release run --dry-run
cargo run -p dhara_tool -- config sync
```

## CI vs local

GitHub Actions ([`.github/workflows/pipeline.yml`](../../.github/workflows/pipeline.yml)) runs `cargo`/`dotnet` directly for quality and tests. Scripts wrap `dhara_tool` for native staging, package verification, and release. See [docs/ci-cd-pipelines.md](../../docs/ci-cd-pipelines.md).

Launching `dhara_tool` without a subcommand in an interactive terminal opens the Dhara TUI (**interactive** mode). Explicit subcommands use **direct** mode (no TUI).

## Logging

Audit logs follow [docs/logging.md](../../docs/logging.md). Each run writes to `tooling/logs/{date}_dhara_tool[_N].log`.

Default logging uses INFO on console and file. Use `-m` / `--min` for WARN-only file logs, or `-t` / `--trace` for DEBUG file detail.

Parallel TrID parse/reduce uses Rayon with a capped global thread pool. Cap workers with `-w` / `--workers` (default 4) or `TOOL_MAX_WORKERS`.

## Output layout

- `src/core/dhara_storage_dal/resources/` — embedded `filedefs.dat` built by defs commands
- `tooling/output/` — NuGet packages and other operator artifacts
- `tooling/logs/` — operator audit logs
- `tooling/artifacts/` — gitignored staging for native staging and local NuGet config during verification

See [docs/filedefs-dat.md](../../docs/filedefs-dat.md) for the DSFD on-disk format.
