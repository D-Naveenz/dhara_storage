# dhara_storage_dal

[![crates.io](https://img.shields.io/crates/v/dhara_storage_dal)](https://crates.io/crates/dhara_storage_dal)
[![docs.rs](https://img.shields.io/docsrs/dhara_storage_dal)](https://docs.rs/dhara_storage_dal)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://github.com/D-Naveenz/dhara_storage/blob/main/LICENSE.txt)

`dhara_storage_dal` is the FlatBuffers-backed data access layer for Dhara Storage file definitions.
It owns the DSFD on-disk layout, schema-generated accessors, and the runtime `filedefs.dat` package consumed by [dhara_storage][repo-dhara-storage] and [dhara_tool][repo-tool].

## ✨ Key Features

- **DSFD container** — fixed header, FlatBuffers payload, XML metadata footer
- **Encode / decode** — round-trip definition packages to and from disk
- **Bundled runtime package** — `resources/filedefs.dat` embedded via `include_bytes!`
- **Shared schema** — one FlatBuffers schema for analysis, tooling, and verification

## 📦 Tech Stack & Architecture

| Piece | Role |
|-------|------|
| FlatBuffers | Compact binary definition records |
| `quick-xml` | XML metadata footer parsing |
| `serde` | Supporting serialization for tooling paths |

```
dhara_storage_dal/
├── schema/filedefs.fbs     # canonical FlatBuffers schema
├── src/generated/          # flatc output
├── resources/filedefs.dat  # runtime package (rebuilt by dhara_tool)
└── src/                    # encode, decode, bundled package loader
```

Full binary layout: [filedefs.dat / DSFD reference][filedefs-dat].

## 🚀 Getting Started & Installation

**Prerequisites:** Rust stable. Most apps depend on [dhara_storage][repo-dhara-storage] instead of this crate directly.

```toml
[dependencies]
dhara_storage_dal = "0.9.0"
```

## 🔧 Configuration & Environment Variables

No runtime environment variables. The embedded package path is fixed at `resources/filedefs.dat` inside this crate.

Refresh the embedded artifact from the workspace:

```powershell
cargo run -p dhara_tool -- defs sync-embedded
```

Regenerate Rust accessors after editing the schema:

```powershell
flatc --rust -o src/core/dhara_storage_dal/src/generated src/core/dhara_storage_dal/schema/filedefs.fbs
```

## 🛠️ Usage Examples

Typical consumption is indirect through `dhara_storage` analysis APIs.
Direct DAL entry points include:

- `encode_definition_package` / `decode_definition_package` — full file round-trip
- `root_definition_package` — borrowed view over an in-memory buffer
- `bundled_definition_package` — compile-time embedded runtime package

Inspect a built package from the workspace:

```powershell
cargo run -p dhara_tool -- defs inspect
```

**Troubleshooting**

- Stale analysis after defs update → run `defs sync-embedded` and rebuild dependents.
- Schema drift → regenerate `src/generated/` with `flatc` after editing `filedefs.fbs`.

## ✅ Testing & Quality Assurance

```powershell
cargo test -p dhara_storage_dal
cargo clippy -p dhara_storage_dal --all-targets -- -D warnings
```

API docs: [docs.rs/dhara_storage_dal][docs-rs].

## 🤝 Contributing & License

Part of the [Dhara Storage workspace][repo-root]. Licensed under Apache-2.0.

Operator build pipeline and audit logs: [filedefs reference][filedefs-dat], [logging conventions][logging].

[repo-root]: https://github.com/D-Naveenz/dhara_storage
[repo-dhara-storage]: https://github.com/D-Naveenz/dhara_storage/tree/main/src/core/dhara_storage
[repo-tool]: https://github.com/D-Naveenz/dhara_storage/tree/main/tooling/dhara_tool
[filedefs-dat]: https://github.com/D-Naveenz/dhara_storage/blob/main/docs/filedefs-dat.md
[logging]: https://github.com/D-Naveenz/dhara_storage/blob/main/docs/logging.md
[docs-rs]: https://docs.rs/dhara_storage_dal
