use crate::error::DefinitionPackageError;
use crate::metadata_xml::{deserialize_metadata, serialize_metadata};
use crate::model::{
    DEFINITION_PACKAGE_IDENTIFIER, DSFD_FILE_HEADER_LEN, DSFD_FORMAT_VERSION, DefinitionPackage,
    DefinitionPackageView,
};

use super::codec::{decode_flatbuffer_payload, encode_flatbuffer_payload, root_flatbuffer_package};

const METADATA_LENGTH_LEN: usize = 4;

/// Encode a full `filedefs.dat` file with header, FlatBuffers payload, and XML footer.
pub fn encode_definition_package(package: &DefinitionPackage) -> Vec<u8> {
    let payload = encode_flatbuffer_payload(package);
    let metadata = serialize_metadata(package);
    assemble_file(&payload, &metadata)
}

/// Decode a full `filedefs.dat` file into an owned definition package.
///
/// # Errors
///
/// Returns an error when the container layout, FlatBuffers payload, or XML metadata
/// is invalid.
pub fn decode_definition_package(
    bytes: &[u8],
) -> Result<DefinitionPackage, DefinitionPackageError> {
    let (payload, metadata_xml) = split_file(bytes)?;
    let mut package = decode_flatbuffer_payload(payload)?;
    deserialize_metadata(metadata_xml, &mut package)?;
    Ok(package)
}

/// Return a verified borrowed FlatBuffers root view over the payload inside a file buffer.
///
/// # Errors
///
/// Returns an error when the container layout is invalid or FlatBuffers verification fails.
pub fn root_definition_package(
    bytes: &[u8],
) -> Result<DefinitionPackageView<'_>, DefinitionPackageError> {
    let (payload, _) = split_file(bytes)?;
    root_flatbuffer_package(payload)
}

fn assemble_file(payload: &[u8], metadata: &[u8]) -> Vec<u8> {
    let payload_length = u32::try_from(payload.len()).expect("payload length should fit in u32");
    let metadata_length = u32::try_from(metadata.len()).expect("metadata length should fit in u32");
    let total_len = DSFD_FILE_HEADER_LEN + payload.len() + METADATA_LENGTH_LEN + metadata.len();
    let mut file = Vec::with_capacity(total_len);
    file.extend_from_slice(DEFINITION_PACKAGE_IDENTIFIER.as_bytes());
    file.extend_from_slice(&DSFD_FORMAT_VERSION.to_le_bytes());
    file.extend_from_slice(&payload_length.to_le_bytes());
    file.extend_from_slice(payload);
    file.extend_from_slice(&metadata_length.to_le_bytes());
    file.extend_from_slice(metadata);
    file
}

fn split_file(bytes: &[u8]) -> Result<(&[u8], &[u8]), DefinitionPackageError> {
    if bytes.len() < DSFD_FILE_HEADER_LEN + METADATA_LENGTH_LEN {
        return Err(DefinitionPackageError::InvalidContainerLayout {
            message: "file is too small to contain a DSFD package".to_owned(),
        });
    }

    if &bytes[..4] != DEFINITION_PACKAGE_IDENTIFIER.as_bytes() {
        return Err(DefinitionPackageError::InvalidMagic);
    }

    let format_version = u16::from_le_bytes([bytes[4], bytes[5]]);
    if format_version != DSFD_FORMAT_VERSION {
        return Err(DefinitionPackageError::UnsupportedFormatVersion {
            version: format_version,
        });
    }

    let payload_length =
        usize::try_from(u32::from_le_bytes([bytes[6], bytes[7], bytes[8], bytes[9]])).map_err(
            |_| DefinitionPackageError::InvalidContainerLayout {
                message: "payload length is invalid".to_owned(),
            },
        )?;

    let metadata_length_offset = DSFD_FILE_HEADER_LEN
        .checked_add(payload_length)
        .ok_or_else(|| DefinitionPackageError::InvalidContainerLayout {
            message: "payload length overflows the file layout".to_owned(),
        })?;
    let metadata_offset = metadata_length_offset
        .checked_add(METADATA_LENGTH_LEN)
        .ok_or_else(|| DefinitionPackageError::InvalidContainerLayout {
            message: "metadata offset overflows the file layout".to_owned(),
        })?;

    if bytes.len() < metadata_offset {
        return Err(DefinitionPackageError::InvalidContainerLayout {
            message: "metadata section is truncated".to_owned(),
        });
    }

    let metadata_length = usize::try_from(u32::from_le_bytes([
        bytes[metadata_length_offset],
        bytes[metadata_length_offset + 1],
        bytes[metadata_length_offset + 2],
        bytes[metadata_length_offset + 3],
    ]))
    .map_err(|_| DefinitionPackageError::InvalidContainerLayout {
        message: "metadata length is invalid".to_owned(),
    })?;

    let expected_total = metadata_offset
        .checked_add(metadata_length)
        .ok_or_else(|| DefinitionPackageError::InvalidContainerLayout {
            message: "file layout overflows".to_owned(),
        })?;
    if bytes.len() != expected_total {
        return Err(DefinitionPackageError::InvalidContainerLayout {
            message: format!(
                "expected file size {expected_total} bytes, found {} bytes",
                bytes.len()
            ),
        });
    }

    let payload_end = DSFD_FILE_HEADER_LEN
        .checked_add(payload_length)
        .ok_or_else(|| DefinitionPackageError::InvalidContainerLayout {
            message: "payload end offset overflows".to_owned(),
        })?;
    let payload = &bytes[DSFD_FILE_HEADER_LEN..payload_end];
    let metadata = &bytes[metadata_offset..metadata_offset + metadata_length];
    Ok((payload, metadata))
}

#[cfg(test)]
mod tests {
    use super::{decode_definition_package, encode_definition_package, root_definition_package};
    use crate::codec::{decode_flatbuffer_payload, encode_flatbuffer_payload};
    use crate::error::DefinitionPackageError;
    use crate::model::{
        DEFINITION_PACKAGE_IDENTIFIER, DefinitionPackage, DefinitionRecord, SignatureDefinition,
        SignaturePattern,
    };

    fn sample_package() -> DefinitionPackage {
        DefinitionPackage {
            package_version: "0.6.0".to_owned(),
            definitions_release: "2026-06-24".to_owned(),
            package_revision: 1,
            tags: 48,
            definitions: vec![DefinitionRecord {
                file_type: "Portable Network Graphics".to_owned(),
                extensions: vec!["png".to_owned()],
                mime_type: "image/png".to_owned(),
                remarks: "fixture".to_owned(),
                signature: SignatureDefinition {
                    patterns: vec![SignaturePattern {
                        position: 0,
                        data: vec![0x89, b'P', b'N', b'G'],
                    }],
                    strings: vec![b"IHDR".to_vec()],
                },
                priority_level: 42,
            }],
        }
    }

    #[test]
    fn dsfd_file_roundtrip_preserves_package_semantics() {
        let package = sample_package();
        let encoded = encode_definition_package(&package);
        let decoded = decode_definition_package(&encoded).expect("package should decode");

        assert_eq!(decoded, package);
    }

    #[test]
    fn root_view_reads_payload_from_full_file() {
        let encoded = encode_definition_package(&sample_package());
        let root = root_definition_package(&encoded).expect("root should verify");

        assert_eq!(root.definitions().unwrap().len(), 1);
        assert_eq!(root.package_revision(), 1);
    }

    #[test]
    fn malformed_file_is_rejected() {
        let error = decode_definition_package(b"not-a-dsfd-package").unwrap_err();

        assert!(matches!(error, DefinitionPackageError::InvalidMagic));
    }

    #[test]
    fn flatbuffer_payload_roundtrip_still_works() {
        let package = sample_package();
        let payload = encode_flatbuffer_payload(&package);
        let decoded = decode_flatbuffer_payload(&payload).expect("payload should decode");

        assert_eq!(decoded.package_revision, package.package_revision);
        assert_eq!(decoded.tags, package.tags);
        assert_eq!(decoded.definitions, package.definitions);
    }

    #[test]
    fn encoded_file_ends_with_xml_closing_tag() {
        let encoded = encode_definition_package(&sample_package());

        assert_eq!(encoded.last(), Some(&b'>'));
        assert_ne!(
            &encoded[encoded.len().saturating_sub(4)..],
            DEFINITION_PACKAGE_IDENTIFIER.as_bytes()
        );
    }

    #[test]
    fn file_magic_is_at_offset_zero_only_in_header() {
        let encoded = encode_definition_package(&sample_package());

        assert_eq!(&encoded[..4], DEFINITION_PACKAGE_IDENTIFIER.as_bytes());
        assert_ne!(
            encoded.last(),
            Some(&DEFINITION_PACKAGE_IDENTIFIER.as_bytes()[3])
        );
    }
}
