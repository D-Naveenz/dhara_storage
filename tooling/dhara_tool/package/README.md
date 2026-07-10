Build input for large operator files that ship beside the `dhara_tool` binary.

- Keep source archives such as `triddefs_xml.7z` here during development.
- Place a sidecar manifest beside each source archive or directory, for example `triddefs_xml.source.toml`:
```toml
definitions_release = "24/06/2026"
```

The builder normalizes the date to ISO `YYYY-MM-DD` in the output `filedefs.dat`.
- During a normal Cargo build, `build.rs` copies this folder into the active
  output directory (`{tool_root}/package/`) beside the compiled binary, similar
  to an MSBuild "Copy to Output Directory" step.
- At runtime, defs commands read from `{tool_root}/package/` by default (not from
  this source path in the repository tree).
- When the CLI runs without an explicit `--input`, it prefers `triddefs_xml.7z`
  when it exists in the shipped `package/` directory.
- Launching `dhara_tool` without a subcommand opens the interactive GUI when a
  graphical display is available; defs commands in that GUI use the same default.

Small compile-time reference catalogs (MIME types, extension seeds) live in
[`crates/dhara_tool_kernel/data/`](../crates/dhara_tool_kernel/data/) and are
embedded into the binary — they are not copied here.

Source: [TrIDNet - File Identifier](https://mark0.net/soft-tridnet-e.html)
