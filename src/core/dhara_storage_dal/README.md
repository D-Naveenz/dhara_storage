# dhara_storage_dal

[![crates.io](https://img.shields.io/crates/v/dhara_storage_dal)](https://crates.io/crates/dhara_storage_dal)
[![docs.rs](https://img.shields.io/docsrs/dhara_storage_dal)](https://docs.rs/dhara_storage_dal)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://github.com/D-Naveenz/dhara_storage/blob/main/LICENSE.txt)

Data access layer for Dhara Storage **file definitions**—the signature data behind content-based type intelligence.

Most applications depend on [`dhara_storage`][runtime] instead of this crate. Use `dhara_storage_dal` when you need to encode, decode, or inspect definition packages directly.

## Why this crate

- DSFD on-disk layout (header, FlatBuffers payload, XML footer)
- Encode / decode definition packages
- Bundled runtime package at `resources/filedefs.dat`

## Prerequisites

- Rust **stable** toolchain (`cargo`)

## Install

```toml
[dependencies]
dhara_storage_dal = "0.9.0"
```

## Usage

### 1. Prefer the runtime for analysis

For typing files in an application, use [`dhara_storage`][runtime] analysis APIs. They consume the bundled package from this crate.

### 2. Work with packages directly

Entry points include:

- `encode_definition_package` / `decode_definition_package` — full file round-trip
- `root_definition_package` — borrowed view over an in-memory buffer
- `bundled_definition_package` — compile-time embedded runtime package

Binary layout details: [filedefs.dat / DSFD][filedefs-dat].

## Related

- Runtime crate: [dhara_storage][runtime]
- Product overview: [Dhara Storage][root]
- API docs: [docs.rs/dhara_storage_dal][docs-rs]

## License

Apache-2.0.

[runtime]: https://github.com/D-Naveenz/dhara_storage/blob/main/src/core/dhara_storage/README.md
[root]: https://github.com/D-Naveenz/dhara_storage
[filedefs-dat]: https://github.com/D-Naveenz/dhara_storage/blob/main/docs/filedefs-dat.md
[docs-rs]: https://docs.rs/dhara_storage_dal
