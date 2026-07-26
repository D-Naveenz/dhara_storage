# `filedefs.dat` — DSFD reference

This document describes the on-disk **Dhara Storage File Definition (DSFD)** package
used for content-based file-type identification. The canonical runtime artifact is
`src/core/dhara_storage/resources/filedefs.dat`. It is built by `drot`, embedded into
`dhara_storage` at compile time, and decoded with `dhara_storage_core`.

## Overview

A DSFD package is a single binary file with three logical regions:

1. **Fixed header** — file magic, format version, and payload length.
2. **FlatBuffers payload** — the definition records used for matching.
3. **XML metadata footer** — human- and tool-readable package metadata.

The file ends naturally on the XML closing `>`. There is no trailing end-of-file magic.

```
+----------+----------+------------------+---------------+------------------+
| DSFD     | header   | FlatBuffers      | metadata_len  | XML metadata     |
| (4 bytes)| fields   | payload (N bytes)| (4 bytes)     | (M bytes, ends >)|
+----------+----------+------------------+---------------+------------------+
 offset 0            offset 10          offset 10+N     offset 14+N
```

Only **one** `DSFD` magic appears in the binary layout (bytes 0–3). The human-readable
signature string inside the XML footer may also mention `DSFD`; that is not a second
file magic.

## Container format (version 2)

| Offset | Size | Field | Type | Description |
|--------|------|-------|------|-------------|
| 0 | 4 | `magic` | ASCII | Always `DSFD` |
| 4 | 2 | `format_version` | `u16` LE | Currently `2` |
| 6 | 4 | `payload_length` | `u32` LE | Byte length of the FlatBuffers section |
| 10 | N | `payload` | bytes | Serialized `DefinitionPackage` root table |
| 10 + N | 4 | `metadata_length` | `u32` LE | Byte length of the XML footer |
| 14 + N | M | `metadata` | UTF-8 XML | Compact metadata document |

**Total file size:** `10 + N + 4 + M` bytes.

**Validation rules:**

- `magic` must be `DSFD`.
- `format_version` must be supported by the reader (`2` today).
- `payload_length` and `metadata_length` must bound their sections without overflow.
- The FlatBuffers root must verify structurally.
- The XML footer must parse and pass semantic checks against the payload.

Version 1 (duplicate `DSFD` markers inside the payload and at EOF) is not supported.

Constants and encode/decode logic live in
[`src/core/dhara_storage_core/src/format/container.rs`](../src/core/dhara_storage_core/src/format/container.rs)
and [`model.rs`](../src/core/dhara_storage_core/src/model/mod.rs).

## FlatBuffers payload

Schema:
[`src/core/dhara_storage_core/schema/filedefs.fbs`](../src/core/dhara_storage_core/schema/filedefs.fbs)

Root table: `DefinitionPackage`

| Field | Type | Stored in payload | Also in XML |
|-------|------|-------------------|-------------|
| `package_revision` | `ushort` | yes | yes (`packageRevision`) |
| `tags` | `uint` | yes | yes (`tags`) |
| `definitions` | `[DefinitionRecord]` | yes | count only (`definitionCount`) |

Each `DefinitionRecord` contains:

| Field | Purpose |
|-------|---------|
| `file_type` | Human-readable type label |
| `extensions` | Known filename extensions |
| `mime_type` | Preferred MIME type |
| `remarks` | Source notes / diagnostics |
| `signature` | Positional byte patterns and extracted strings |
| `priority_level` | Relative ranking when multiple definitions match |

**Not stored in FlatBuffers** (XML footer only):

| Field | XML element | Meaning |
|-------|-------------|---------|
| `package_version` | `packageVersion` | `dhara_storage_core` semver (DSFD packaging authority) |
| `definitions_release` | `definitionsRelease` | ISO `YYYY-MM-DD` date of the upstream dataset |

The payload section does not use a FlatBuffers `file_identifier`. Section boundaries
are defined entirely by `payload_length` in the file header.

Regenerate Rust accessors after editing the schema:

```powershell
flatc --rust -o src/core/dhara_storage_core/src/generated src/core/dhara_storage_core/schema/filedefs.fbs
```

## XML metadata footer

The footer is a single-line XML document prefixed by a standard XML declaration.
Example shape:

```xml
<?xml version="1.0" encoding="UTF-8"?><dsfd xmlns="https://raw.githubusercontent.com/D-Naveenz/dhara_storage/main/src/core/dhara_storage_core/schema/dsfd-metadata.xsd"><signature>Dhara Storage File Definition package - DSFD</signature><packageVersion>0.8.0</packageVersion><definitionsRelease>2026-06-24</definitionsRelease><packageRevision>1</packageRevision><tags>48</tags><definitionCount>5500</definitionCount></dsfd>
```

### Schema (XSD)

Machine-readable schema:
[`src/core/dhara_storage_core/schema/dsfd-metadata.xsd`](../src/core/dhara_storage_core/schema/dsfd-metadata.xsd)

The `xmlns` attribute on the root `dsfd` element must match `DSFD_METADATA_XMLNS` in
[`model.rs`](../src/core/dhara_storage_core/src/model/mod.rs). That constant is a raw
GitHub URL to the XSD on the default branch. Local tools validate against the
checked-in XSD file; the URL is for external consumers once the file is published.

### Semantic cross-checks

When decoding, the DAL verifies that XML metadata is consistent with the FlatBuffers
payload:

- `packageRevision` matches `package_revision` in the payload.
- `tags` matches the payload.
- `definitionCount` matches `definitions.len()`.
- `signature` matches the expected human-readable DSFD signature string.

`packageVersion` and `definitionsRelease` are taken from XML and merged into the owned
`DefinitionPackage` model.

## `packageRevision` semantics

`packageRevision` is a **per-packaging-version build counter**, not a global lifetime
counter. `drot` assigns it when building from TrID sources.

| Existing `filedefs.dat` | `packageVersion` vs current core | Next revision |
|-------------------------|----------------------------------|---------------|
| Missing or invalid | — | `1` |
| Present | matches current core version | `existing + 1` |
| Present | differs from current core version | `1` |

`packageVersion` is provenance metadata only. A differing label alone does not mean the
embedded payload is stale — `defs sync-embedded` compares definition content and
`definitionsRelease`, not version strings.

Example: three rebuilds at DAL `0.6.0` produce revisions `1`, `2`, `3`. After a DAL
version bump to `0.8.0`, the next build starts again at `1`.

At startup, `drot` reads the canonical output path, caches revision and version
for logging and the GUI workspace snapshot, and updates the cache after each successful write.
See [`tooling/drot/src/drot_kernel/src/workspace.rs`](../tooling/drot/src/drot_kernel/src/workspace.rs).

## `tags` field

`tags` is a builder-defined `u32` bitfield. The TrID XML pipeline currently writes
`48` (`VALIDATED_TAGS` in `drot`). Treat unspecified bits as reserved for
future builder features.

## Build pipeline and artifact locations

| Path | Role |
|------|------|
| `tooling/drot/src/drot_dhara_storage/package/triddefs_xml.7z` | Build input: TrID XML source archive (gitignored when large) |
| `tooling/drot/src/drot_dhara_storage/package/triddefs_xml.source.toml` | Build input: sidecar with upstream `definitions_release` date |
| `{tool_root}/package/` | Runtime default for TrID input (copied beside binary at build) |
| `src/core/dhara_storage/resources/filedefs.dat` | Embedded runtime package (published with crate) |
| `dhara_storage` (compile time) | Embeds `resources/filedefs.dat` via `include_bytes!` |
| `tooling/drot/src/drot_dhara_storage/data/` | Compile-time MIME/extension catalogs (`include_str!`) |

Typical operator commands:

```powershell
# Build from the default TrID archive into src/core/dhara_storage/resources/filedefs.dat
cargo run --manifest-path tooling/drot/Cargo.toml -p drot -- defs build-trid-xml -v

# Inspect the current package
cargo run --manifest-path tooling/drot/Cargo.toml -p drot -- defs inspect

# Re-copy / rebuild the embedded runtime artifact when needed
cargo run --manifest-path tooling/drot/Cargo.toml -p drot -- defs sync-embedded
```

The sidecar TOML uses the source stem (`triddefs_xml.source.toml` beside
`triddefs_xml.7z`). Dates may be written as `YYYY-MM-DD` or `DD/MM/YYYY`; the builder
normalizes to ISO `YYYY-MM-DD` in the output metadata.

## Code map

| Crate / module | Responsibility |
|----------------|----------------|
| `dhara_storage_core` | Owns DSFD layout, FlatBuffers schema, XML metadata, encode/decode (no embed) |
| `dhara_storage` | Runtime analysis; embeds and indexes `filedefs.dat` |
| `drot` | Builds, inspects, syncs, and assigns `packageRevision` |
| `dharastorage` | C ABI for managed hosts; does not parse DSFD layout directly |

Public core entry points:

- `encode_definition_package` / `decode_definition_package` — full file round-trip
- `root_definition_package` — borrowed view over payload inside a file buffer

Runtime embed entry point (`dhara_storage`):

- `bundled_definition_package` — compile-time embedded `filedefs.dat`
## Design notes

**Why XML at the end?** Metadata such as tool version and dataset release date is
operator-facing and easy to inspect without a FlatBuffers decoder. Length-prefixed
sections plus XML parsing provide enough structure without a redundant EOF marker.

**Why FlatBuffers for definitions?** The definition set is large (thousands of records
with byte patterns). FlatBuffers supports compact binary storage and zero-copy style
access patterns suitable for an embedded runtime package.

**Why split metadata across payload and XML?** Fields needed for fast structural
validation and matching (`package_revision`, `tags`, records) stay in the binary
payload. Fields that describe provenance and build context (`package_version`,
`definitions_release`) live in XML where tooling and humans can read them directly.

## Related docs

- [Logging conventions][logging] — audit log format for `drot` builds
- [dhara_storage_core README][readme-core] — crate-local quick reference
- [drot package/ notes][package-readme] — shipped TrID build inputs
- [CI/CD pipelines][ci-cd] — defs build in release flow
- [Docs index][docs-index]

[logging]: logging.md
[readme-core]: ../src/core/dhara_storage_core/README.md
[package-readme]: ../tooling/drot/src/drot_dhara_storage/package/README.md
[ci-cd]: ci-cd-pipelines.md
[docs-index]: README.md
