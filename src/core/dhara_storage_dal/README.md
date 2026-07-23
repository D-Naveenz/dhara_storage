# dhara_storage_dal

[![crates.io](https://img.shields.io/crates/v/dhara_storage_dal)](https://crates.io/crates/dhara_storage_dal)
[![docs.rs](https://img.shields.io/docsrs/dhara_storage_dal)](https://docs.rs/dhara_storage_dal)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://github.com/D-Naveenz/dhara_storage/blob/main/LICENSE.txt)

`dhara_storage_dal` is the data access layer for Dhara Storage **file definitions**—the signature data that powers content-based type intelligence.

It owns the DSFD on-disk layout, FlatBuffers accessors, and the embedded `filedefs.dat` package used at runtime by [dhara_storage][repo-dhara-storage].

Most applications depend on `dhara_storage` instead of this crate directly.

## Install

```toml
[dependencies]
dhara_storage_dal = "0.9.0"
```

## What you get

- Encode / decode definition packages (DSFD: header, FlatBuffers payload, XML footer)
- Bundled runtime package at `resources/filedefs.dat`
- Shared schema for analysis and operator tooling

Layout details: [filedefs.dat / DSFD reference][filedefs-dat].

## Usage

Typical path: call analysis APIs on [dhara_storage][repo-dhara-storage]. Direct DAL entry points include `encode_definition_package`, `decode_definition_package`, `root_definition_package`, and `bundled_definition_package`.

## License

Apache-2.0. Part of the [Dhara Storage workspace][repo-root].

[repo-root]: https://github.com/D-Naveenz/dhara_storage
[repo-dhara-storage]: https://github.com/D-Naveenz/dhara_storage/tree/main/src/core/dhara_storage
[filedefs-dat]: https://github.com/D-Naveenz/dhara_storage/blob/main/docs/filedefs-dat.md
