use std::collections::BTreeSet;

use dhara_dhbin::{DhbinDecodeError, DhbinReadOptions, DhbinReader};
use once_cell::sync::Lazy;
use serde::de::IgnoredAny;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{debug, info};

use crate::error::StorageError;

const BUNDLED_FILEDEFS_BYTES: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/resources/filedefs.dhbin"
));
/// Four-byte `DHBIN` package identifier used by normalized file-definition packages.
pub const DEFINITION_PACKAGE_ID: [u8; 4] = *b"FDEF";
const CATCH_ALL_INDEX: usize = 256;

static PACKAGE: Lazy<Result<DefinitionPackage, String>> = Lazy::new(|| {
    DefinitionPackage::from_bytes(BUNDLED_FILEDEFS_BYTES).map_err(|err| err.to_string())
});
static DATABASE: Lazy<Result<DefinitionDatabase, String>> = Lazy::new(|| {
    bundled_definition_package()
        .map(DefinitionDatabase::from_package)
        .map_err(|err| err.to_string())
});

/// Errors that can occur while decoding a file-definition package.
#[derive(Debug, Error)]
pub enum DefinitionPackageDecodeError {
    /// The outer `DHBIN` container could not be parsed or validated.
    #[error("failed to decode DHBIN container: {0}")]
    Dhbin(#[from] DhbinDecodeError),

    /// The inner MessagePack payload could not be deserialized into the expected schema.
    #[error("failed to decode MessagePack payload: {0}")]
    MessagePack(#[from] rmp_serde::decode::Error),

    /// The package advertised a four-byte identifier other than `FDEF`.
    #[error("unexpected package identifier '{found}'")]
    InvalidPackageId {
        /// The decoded four-byte identifier rendered as text for diagnostics.
        found: String,
    },
}

/// Serialized file-definition package loaded from `filedefs.dhbin`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefinitionPackage {
    /// The version of the normalized package schema produced by the builder.
    pub package_version: String,
    /// The upstream source-data version carried through from the TrID source set.
    pub source_version: String,
    /// Monotonic package revision assigned by the builder.
    pub package_revision: u16,
    /// Builder-defined package flags.
    pub tags: u32,
    /// All normalized type definitions contained in the package.
    pub definitions: Vec<DefinitionRecord>,
}

/// Single normalized file-type definition record.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DefinitionRecord {
    /// Human-readable label for the detected file type.
    pub file_type: String,
    #[serde(default)]
    /// Known filename extensions associated with the type.
    pub extensions: Vec<String>,
    #[serde(default)]
    /// Preferred MIME type associated with the type.
    pub mime_type: String,
    #[serde(default)]
    /// Additional human-readable notes captured from the source dataset.
    pub remarks: String,
    #[serde(default)]
    /// Signature patterns and extracted strings used for content matching.
    pub signature: SignatureDefinition,
    #[serde(default)]
    /// Relative ranking hint used when multiple definitions match.
    pub priority_level: i32,
}

/// Signature material used to identify a file type from file bytes.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignatureDefinition {
    #[serde(default)]
    /// Positional byte patterns that must match specific file offsets.
    pub patterns: Vec<SignaturePattern>,
    #[serde(default)]
    /// Raw strings captured from the source definitions for diagnostics or future matching work.
    pub strings: Vec<Vec<u8>>,
}

/// Byte sequence that should match at a specific offset within a file.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignaturePattern {
    /// Zero-based byte offset where the pattern should be evaluated.
    pub position: u16,
    #[serde(default)]
    /// The expected byte sequence at `position`.
    pub data: Vec<u8>,
}

#[derive(Debug, Deserialize)]
struct RawPackage(String, String, u16, IgnoredAny, u32, Vec<DefinitionRecord>);

/// Indexed in-memory database used by the analysis engine to narrow definition candidates quickly.
#[derive(Debug, Clone)]
pub struct DefinitionDatabase {
    definitions: Vec<DefinitionRecord>,
    pattern_index: Vec<Vec<usize>>,
}

impl DefinitionPackage {
    /// Decodes a raw MessagePack payload into a normalized definition package.
    ///
    /// # Arguments
    ///
    /// - `bytes` (`&[u8]`) - The MessagePack payload bytes from a definition package.
    ///
    /// # Returns
    ///
    /// - `Result<Self, rmp_serde::decode::Error>` - The decoded package schema.
    pub fn from_messagepack_bytes(bytes: &[u8]) -> Result<Self, rmp_serde::decode::Error> {
        let RawPackage(package_version, source_version, package_revision, _, tags, definitions): RawPackage =
            rmp_serde::from_slice(bytes)?;
        Ok(Self {
            package_version,
            source_version,
            package_revision,
            tags,
            definitions,
        })
    }

    /// Decodes a full `filedefs.dhbin` container using the package's default read behavior.
    ///
    /// # Arguments
    ///
    /// - `bytes` (`&[u8]`) - The raw `DHBIN` bytes to decode.
    ///
    /// # Returns
    ///
    /// - `Result<Self, DefinitionPackageDecodeError>` - The decoded package.
    ///
    /// # Errors
    ///
    /// Returns an error when the container is malformed, the package identifier is not `FDEF`,
    /// or the MessagePack payload cannot be deserialized.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, DefinitionPackageDecodeError> {
        let header = DhbinReader::read_header(bytes)?;
        if header.package_id != DEFINITION_PACKAGE_ID {
            return Err(DefinitionPackageDecodeError::InvalidPackageId {
                found: String::from_utf8_lossy(&header.package_id).into_owned(),
            });
        }
        let payload = DhbinReader::read_payload_bytes(bytes)?;
        Self::from_messagepack_bytes(&payload).map_err(Into::into)
    }

    /// Decodes a full `filedefs.dhbin` container using explicit `DHBIN` read options.
    ///
    /// # Arguments
    ///
    /// - `bytes` (`&[u8]`) - The raw `DHBIN` bytes to decode.
    /// - `options` (`&DhbinReadOptions`) - Reader options that control metadata loading and integrity verification.
    ///
    /// # Returns
    ///
    /// - `Result<Self, DefinitionPackageDecodeError>` - The decoded package.
    ///
    /// # Errors
    ///
    /// Returns an error when the container is malformed, fails validation under `options`,
    /// or does not carry the expected `FDEF` package identifier.
    pub fn from_bytes_with_options(
        bytes: &[u8],
        options: &DhbinReadOptions,
    ) -> Result<Self, DefinitionPackageDecodeError> {
        let package = DhbinReader::read_package(bytes, options)?;
        if package.header.package_id != DEFINITION_PACKAGE_ID {
            return Err(DefinitionPackageDecodeError::InvalidPackageId {
                found: String::from_utf8_lossy(&package.header.package_id).into_owned(),
            });
        }
        Self::from_messagepack_bytes(&package.payload).map_err(Into::into)
    }
}

impl DefinitionDatabase {
    fn from_package(package: &DefinitionPackage) -> Self {
        let definitions = package.definitions.clone();
        let mut pattern_index = vec![Vec::new(); CATCH_ALL_INDEX + 1];
        for (idx, definition) in definitions.iter().enumerate() {
            if definition.signature.patterns.is_empty() {
                pattern_index[CATCH_ALL_INDEX].push(idx);
                continue;
            }

            for pattern in &definition.signature.patterns {
                if let Some(first_byte) = pattern.data.first() {
                    pattern_index[*first_byte as usize].push(idx);
                } else {
                    pattern_index[CATCH_ALL_INDEX].push(idx);
                }
            }
        }

        Self {
            definitions,
            pattern_index,
        }
    }

    pub(crate) fn candidate_indices(&self, header: &[u8]) -> Vec<usize> {
        let mut candidates = BTreeSet::new();

        for idx in &self.pattern_index[CATCH_ALL_INDEX] {
            candidates.insert(*idx);
        }

        for (position, byte) in header.iter().enumerate() {
            for definition_idx in &self.pattern_index[*byte as usize] {
                let definition = &self.definitions[*definition_idx];
                if definition
                    .signature
                    .patterns
                    .iter()
                    .any(|pattern| pattern.position as usize == position)
                {
                    candidates.insert(*definition_idx);
                }
            }
        }

        candidates.into_iter().collect()
    }

    pub(crate) fn definition(&self, idx: usize) -> &DefinitionRecord {
        &self.definitions[idx]
    }
}

/// Returns the embedded file-definition package bundled with the crate.
///
/// # Returns
///
/// - `Result<&'static DefinitionPackage, StorageError>` - The lazily decoded embedded package.
///
/// # Errors
///
/// Returns an error when the embedded `filedefs.dhbin` asset cannot be decoded.
pub fn bundled_definition_package() -> Result<&'static DefinitionPackage, StorageError> {
    debug!(target: "dhara_storage::definitions", "loading bundled definition package");
    PACKAGE
        .as_ref()
        .map_err(|message| StorageError::DefinitionsLoad {
            message: message.clone(),
        })
}

/// Decodes an in-memory `filedefs.dhbin` blob using default read options.
///
/// # Arguments
///
/// - `bytes` (`&[u8]`) - The raw package bytes to decode.
///
/// # Returns
///
/// - `Result<DefinitionPackage, StorageError>` - The decoded file-definition package.
///
/// # Errors
///
/// Returns an error when the package cannot be parsed, validated, or deserialized.
pub fn decode_definition_package(bytes: &[u8]) -> Result<DefinitionPackage, StorageError> {
    info!(
        target: "dhara_storage::definitions",
        byte_len = bytes.len(),
        "decoding definition package with default options"
    );
    decode_definition_package_with_options(bytes, &DhbinReadOptions::default())
}

/// Decodes an in-memory `filedefs.dhbin` blob using caller-provided `DHBIN` read options.
///
/// # Arguments
///
/// - `bytes` (`&[u8]`) - The raw package bytes to decode.
/// - `options` (`&DhbinReadOptions`) - Reader options that control metadata loading and integrity verification.
///
/// # Returns
///
/// - `Result<DefinitionPackage, StorageError>` - The decoded file-definition package.
///
/// # Errors
///
/// Returns an error when the package cannot be parsed, validated, or deserialized under `options`.
pub fn decode_definition_package_with_options(
    bytes: &[u8],
    options: &DhbinReadOptions,
) -> Result<DefinitionPackage, StorageError> {
    info!(
        target: "dhara_storage::definitions",
        byte_len = bytes.len(),
        verify_integrity = options.verify_integrity,
        load_metadata = options.load_metadata,
        "decoding definition package"
    );
    DefinitionPackage::from_bytes_with_options(bytes, options).map_err(|err| {
        StorageError::DefinitionsLoad {
            message: err.to_string(),
        }
    })
}

/// Decodes a raw MessagePack definition-package payload without an outer `DHBIN` container.
///
/// # Arguments
///
/// - `bytes` (`&[u8]`) - The raw MessagePack payload bytes.
///
/// # Returns
///
/// - `Result<DefinitionPackage, StorageError>` - The decoded file-definition package.
///
/// # Errors
///
/// Returns an error when the MessagePack payload does not match the expected schema.
pub fn decode_definition_package_payload(bytes: &[u8]) -> Result<DefinitionPackage, StorageError> {
    debug!(
        target: "dhara_storage::definitions",
        byte_len = bytes.len(),
        "decoding raw definition package payload"
    );
    DefinitionPackage::from_messagepack_bytes(bytes).map_err(|err| StorageError::DefinitionsLoad {
        message: err.to_string(),
    })
}

pub(crate) fn database() -> Result<&'static DefinitionDatabase, StorageError> {
    DATABASE
        .as_ref()
        .map_err(|message| StorageError::DefinitionsLoad {
            message: message.clone(),
        })
}

#[cfg(test)]
mod tests {
    use dhara_dhbin::{
        CompressionKind, DHBIN_MAGIC, DhbinReadOptions, DhbinWriteOptions, DhbinWriter,
        IntegrityKind, PackagePurpose,
    };
    use serde::Serialize;

    use super::{
        DEFINITION_PACKAGE_ID, DefinitionPackage, bundled_definition_package, database,
        decode_definition_package, decode_definition_package_payload,
    };

    #[derive(Serialize)]
    struct RawPackageOut(String, String, u16, (), u32, Vec<super::DefinitionRecord>);

    fn encode_test_package(package: &DefinitionPackage, purpose: PackagePurpose) -> Vec<u8> {
        let payload = rmp_serde::to_vec(&RawPackageOut(
            package.package_version.clone(),
            package.source_version.clone(),
            package.package_revision,
            (),
            package.tags,
            package.definitions.clone(),
        ))
        .unwrap();
        DhbinWriter::write_payload_bytes(
            &payload,
            &DhbinWriteOptions {
                package_id: DEFINITION_PACKAGE_ID,
                purpose,
                compression: CompressionKind::Lz4Frame,
                flags: 0,
                metadata: Some(rmp_serde::to_vec(&"filedefs").unwrap()),
                integrity: IntegrityKind::Sha256,
            },
        )
        .unwrap()
    }

    #[test]
    fn bundled_dhbin_loads_successfully() {
        let package =
            bundled_definition_package().expect("bundled definitions package should deserialize");
        assert!(!package.definitions.is_empty());
    }

    #[test]
    fn png_header_returns_png_candidate() {
        let db = database().expect("bundled definitions package should deserialize");
        let header = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

        let candidates = db
            .candidate_indices(&header)
            .into_iter()
            .map(|idx| db.definition(idx))
            .collect::<Vec<_>>();

        assert!(!candidates.is_empty());
        assert!(candidates.iter().any(|definition| {
            definition
                .extensions
                .iter()
                .any(|ext| ext.eq_ignore_ascii_case("png") || ext.eq_ignore_ascii_case(".png"))
        }));
    }

    #[test]
    fn builder_style_package_roundtrip_is_semantically_stable() {
        let package =
            bundled_definition_package().expect("bundled definitions package should deserialize");
        let bytes = encode_test_package(package, PackagePurpose::Standard);
        let decoded = decode_definition_package(&bytes).expect("encoded package should decode");

        assert_eq!(&decoded, package);
    }

    #[test]
    fn encoded_package_uses_dhbin_header() {
        let package =
            bundled_definition_package().expect("bundled definitions package should deserialize");
        let bytes = encode_test_package(package, PackagePurpose::Standard);

        assert!(bytes.starts_with(DHBIN_MAGIC));
    }

    #[test]
    fn legacy_plain_messagepack_is_rejected() {
        let package =
            bundled_definition_package().expect("bundled definitions package should deserialize");
        let plain = rmp_serde::to_vec(&RawPackageOut(
            package.package_version.clone(),
            package.source_version.clone(),
            package.package_revision,
            (),
            package.tags,
            package.definitions.clone(),
        ))
        .unwrap();

        let error = DefinitionPackage::from_bytes(&plain).expect_err("legacy package should fail");
        assert!(error.to_string().contains("DHBIN"));
    }

    #[test]
    fn wrong_package_id_is_rejected() {
        let package =
            bundled_definition_package().expect("bundled definitions package should deserialize");
        let payload = rmp_serde::to_vec(&RawPackageOut(
            package.package_version.clone(),
            package.source_version.clone(),
            package.package_revision,
            (),
            package.tags,
            package.definitions.clone(),
        ))
        .unwrap();
        let bytes = DhbinWriter::write_payload_bytes(
            &payload,
            &DhbinWriteOptions {
                package_id: *b"CONF",
                purpose: PackagePurpose::Standard,
                compression: CompressionKind::None,
                flags: 0,
                metadata: Some(rmp_serde::to_vec(&"meta").unwrap()),
                integrity: IntegrityKind::None,
            },
        )
        .unwrap();

        let error = DefinitionPackage::from_bytes(&bytes).expect_err("package id should fail");
        assert!(error.to_string().contains("identifier"));
    }

    #[test]
    fn decode_payload_helper_reads_raw_messagepack_payload() {
        let package =
            bundled_definition_package().expect("bundled definitions package should deserialize");
        let payload = rmp_serde::to_vec(&RawPackageOut(
            package.package_version.clone(),
            package.source_version.clone(),
            package.package_revision,
            (),
            package.tags,
            package.definitions.clone(),
        ))
        .unwrap();
        let decoded = decode_definition_package_payload(&payload).expect("payload should decode");
        assert_eq!(decoded.tags, package.tags);
        assert_eq!(decoded.definitions.len(), package.definitions.len());
    }

    #[test]
    fn fast_profile_filedefs_can_still_be_decoded_when_metadata_is_requested() {
        let package =
            bundled_definition_package().expect("bundled definitions package should deserialize");
        let bytes = encode_test_package(package, PackagePurpose::Embedded);
        let decoded = super::decode_definition_package_with_options(
            &bytes,
            &DhbinReadOptions {
                verify_integrity: true,
                load_metadata: true,
            },
        )
        .expect("package should decode");
        assert_eq!(decoded.package_version, package.package_version);
    }
}
