# dhara_storage_core

[![crates.io](https://img.shields.io/crates/v/dhara_storage_core)](https://crates.io/crates/dhara_storage_core)
[![docs.rs](https://img.shields.io/docsrs/dhara_storage_core)](https://docs.rs/dhara_storage_core)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://github.com/D-Naveenz/dhara_storage/blob/main/LICENSE.txt)

Framework crate for Dhara Storage — the abstraction layer that [`dhara_storage`][runtime] builds on.

Most applications depend on the runtime. Use this crate when you need framework primitives directly: definition packages, progress/cancellation, and portable value types. The compile-time embedded `filedefs.dat` asset and path-based handles live in the runtime crate, not here.

## Why this crate

- Shared foundation for the storage runtime (not a thin rename of a data-access layer)
- Definition packages: DSFD on-disk layout, encode / decode, owned model
- Process primitives: progress snapshots, cooperative cancellation, progress reporters
- Portable types: transfer/read/write options, size helpers, attribute and permission value shapes
- Packaging authority semver (`PACKAGE_VERSION`) for tooling

## Prerequisites

- Rust **stable** toolchain (`cargo`)

## Install

```toml
[dependencies]
dhara_storage_core = "0.9.24"
```

## Usage

### 1. Prefer the runtime for applications

For typing files and storage handles, use [`dhara_storage`][runtime]. It embeds definitions, re-exports these framework types, and owns the business APIs.

### 2. Work with definition packages

Entry points include:

- `encode_definition_package` / `decode_definition_package` — full file round-trip
- `root_definition_package` — borrowed view over an in-memory buffer

Binary layout details: [filedefs.dat / DSFD][filedefs-dat].

### 3. Use process and type primitives

- `StorageProgress`, `StorageCancellationToken`, `ProgressReporter`
- `TransferOptions`, `ReadOptions`, `WriteOptions`, `DirectoryDeleteOptions`
- `StorageSize`, `StorageAttributes`, `StoragePermissions` (value shapes; filesystem apply/load stays in the runtime)

## Related

- Runtime crate: [dhara_storage][runtime]
- Product overview: [Dhara Storage][root]
- API docs: [docs.rs/dhara_storage_core][docs-rs]

## License

Apache-2.0.

[runtime]: https://github.com/D-Naveenz/dhara_storage/blob/main/core/dhara_storage/README.md
[root]: https://github.com/D-Naveenz/dhara_storage
[filedefs-dat]: https://github.com/D-Naveenz/dhara_storage/blob/main/docs/filedefs-dat.md
[docs-rs]: https://docs.rs/dhara_storage_core
