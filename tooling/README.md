# tooling

Operator and local-dev tooling for this workspace.

| Path | Role |
|------|------|
| [`drot/`](drot/) | [DROT](https://github.com/D-Naveenz/dhara_repo_orchestration) submodule — CLI/TUI for config, packaging, defs, release |
| [`scripts/`](scripts/) | Local helpers (`ensure-drot-dist`, `verify-local`, SSH/LFS setup) |

DROT is independently versioned; this repo pins it via the submodule gitlink. See [AGENTS.md](../AGENTS.md) for commands.
