Compile-time reference data for the TrID definition builder.

Files here are embedded into the `dhara_tool` binary via `include_str!` at build time.
They are not read from disk at runtime and are not copied beside the executable.

| Path | Purpose |
|------|---------|
| `mime/` | IANA and custom MIME type catalogs |
| `extensions/` | Extension priority seed lists (levels 1–5) |

For large operator inputs (TrID XML archives), use the shipped `package/` directory
beside the executable — see [package/README.md](../../../package/README.md).
