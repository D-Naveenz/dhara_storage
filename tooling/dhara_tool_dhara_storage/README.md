# dhara_tool_dhara_storage

`dhara_tool_dhara_storage` is the repository-specific capability pack that powers
`dhara_tool` for this workspace.

It owns:

- TrID XML ingestion from `.xml`, extracted directories, and `.7z` archives
- normalization and validation of the `filedefs.dhbin` runtime package
- embedded package sync for the runtime crate
- repo config and version synchronization
- NuGet package verification and publish flows for `Dhara.Storage`

## Default Working Folders

When `dhara_tool defs ...` runs against this capability pack, it uses these
repo-relative defaults:

- `package/` for source discovery
- `output/` for generated package files
- `logs/` for dated log files such as `2026-04-10_def_builder.log`

The local `package/` folder inside this crate is excluded from Cargo package
publishing and copied beside built artifacts during a normal build.

## Logging

The capability pack now records richer diagnostic logs for:

- defs command dispatch and effective options
- builder progress and output statistics
- package staging, verification, and publish milestones
- unsupported runtime verification during NuGet validation

This makes the `logs/` output useful for real release diagnostics rather than
only start-or-fail breadcrumbs.

## Related Docs

- Workspace root: <https://github.com/D-Naveenz/dhara_storage#readme>
- Tool shell: <https://github.com/D-Naveenz/dhara_storage/tree/main/tooling/dhara_tool>
