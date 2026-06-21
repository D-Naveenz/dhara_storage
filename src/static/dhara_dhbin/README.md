# dhara_dhbin

[![crates.io](https://img.shields.io/crates/v/dhara_dhbin)](https://crates.io/crates/dhara_dhbin)

`dhara_dhbin` is the shared `DHBIN` v2 container crate used across the Dhara workspace.

It stores MessagePack payloads together with:

- optional MessagePack metadata
- optional integrity data
- explicit package purposes that guide default read behavior
- optional payload compression

## Install

```toml
[dependencies]
dhara_dhbin = "0.4.4"
```

## Quick Start

```rust
use dhara_dhbin::{
    CompressionKind, IntegrityKind, PackagePurpose, DhbinReader, DhbinWriteOptions, DhbinWriter,
};

let bytes = DhbinWriter::write_payload(
    &vec!["alpha".to_string(), "beta".to_string()],
    &DhbinWriteOptions {
        package_id: *b"CONF",
        purpose: PackagePurpose::Standard,
        compression: CompressionKind::Lz4Frame,
        flags: 0,
        metadata: None,
        integrity: IntegrityKind::Sha256,
    },
)?;

let decoded: Vec<String> = DhbinReader::decode_payload(&bytes)?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Read Profiles

- `PackagePurpose::Standard`: verify integrity and load metadata by default
- `PackagePurpose::FastPayload`: favor low-overhead payload access
- `PackagePurpose::Embedded`: favor lightweight embedded runtime reads

## Intended Use

`dhara_dhbin` is intentionally generic. It does not know about TrID definitions,
Dhara metadata models, or any specific runtime payload schema. Those concerns live
in higher-level crates such as `dhara_storage` and `dhara_storage_ops`.
