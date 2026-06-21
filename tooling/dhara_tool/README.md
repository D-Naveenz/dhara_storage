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

Repository-specific command implementations live in `tooling/dhara_storage_ops`.
