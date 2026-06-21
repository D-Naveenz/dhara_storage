This folder is for local builder input assets that are too large or awkward to
spread across the normal source tree.

- Keep source archives such as `triddefs_xml.7z` here.
- The folder is excluded from `cargo package` publishing via `exclude = ["/package"]`.
- During a normal Cargo build, `build.rs` copies this folder into the active
  output directory so the builder executable can find the same assets beside the
  compiled binary, similar to an MSBuild "Copy to Output Directory" step.
- When the CLI runs without an explicit `--input`, it looks here first and
  prefers `triddefs_xml.7z` when it exists.
- Launching `dhara_tool` without a subcommand opens the interactive Dhara TUI
  when a real terminal is available, and the defs commands in that TUI use this
  copied `package/` directory as their default source location.

Source: [TrIDNet - File Identifier](https://mark0.net/soft-tridnet-e.html)
