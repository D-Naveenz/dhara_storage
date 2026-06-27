#![deny(missing_docs)]

//! FlatBuffers data access layer for Dhara Storage file definitions.
//!
//! This crate owns the internal `filedefs.dat` artifact format and provides
//! owned model types plus serializer/deserializer helpers for the runtime and
//! repository tooling crates.

use once_cell::sync::Lazy;
use thiserror::Error;
use tracing::debug;

/// Generated FlatBuffers accessors.
pub mod generated {
    #![allow(clippy::missing_safety_doc)]
    #![allow(missing_docs)]
    include!("generated/filedefs_generated.rs");
}

use generated::dhara::storage::dal as fb;

const BUNDLED_FILEDEFS_BYTES: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../tooling/output/filedefs.dat"
));

static PACKAGE: Lazy<Result<DefinitionPackage, String>> =
    Lazy::new(|| decode_definition_package(BUNDLED_FILEDEFS_BYTES).map_err(|err| err.to_string()));

/// Four-byte FlatBuffers file identifier used by definition packages.
pub const DEFINITION_PACKAGE_IDENTIFIER: &str = fb::DEFINITION_PACKAGE_IDENTIFIER;

/// Default file name for embedded file-definition packages.
pub const FILEDEFS_DAT_FILE_NAME: &str = "filedefs.dat";

/// Borrowed FlatBuffers root view over a definition package.
pub type DefinitionPackageView<'a> = fb::DefinitionPackage<'a>;

/// Errors returned while encoding or decoding definition packages.
#[derive(Debug, Error)]
pub enum DefinitionPackageError {
    /// The buffer does not carry the expected FlatBuffers `FDEF` identifier.
    #[error("definition package does not use the expected FDEF FlatBuffers identifier")]
    InvalidIdentifier,

    /// FlatBuffers verification failed.
    #[error("definition package FlatBuffer is invalid: {0}")]
    InvalidFlatbuffer(#[from] flatbuffers::InvalidFlatbuffer),

    /// The bundled package could not be loaded.
    #[error("failed to load bundled definition package: {message}")]
    BundledLoad {
        /// Decode or validation failure message.
        message: String,
    },
}

/// Serialized file-definition package loaded from `filedefs.dat`.
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
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DefinitionRecord {
    /// Human-readable label for the detected file type.
    pub file_type: String,
    /// Known filename extensions associated with the type.
    pub extensions: Vec<String>,
    /// Preferred MIME type associated with the type.
    pub mime_type: String,
    /// Additional human-readable notes captured from the source dataset.
    pub remarks: String,
    /// Signature patterns and extracted strings used for content matching.
    pub signature: SignatureDefinition,
    /// Relative ranking hint used when multiple definitions match.
    pub priority_level: i32,
}

/// Signature material used to identify a file type from file bytes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SignatureDefinition {
    /// Positional byte patterns that must match specific file offsets.
    pub patterns: Vec<SignaturePattern>,
    /// Raw strings captured from the source definitions for diagnostics or future matching work.
    pub strings: Vec<Vec<u8>>,
}

/// Byte sequence that should match at a specific offset within a file.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SignaturePattern {
    /// Zero-based byte offset where the pattern should be evaluated.
    pub position: u16,
    /// The expected byte sequence at `position`.
    pub data: Vec<u8>,
}

/// Encode an owned definition package into FlatBuffers bytes.
///
/// The output includes the `FDEF` file identifier and is suitable for writing
/// to `filedefs.dat`.
pub fn encode_definition_package(package: &DefinitionPackage) -> Vec<u8> {
    let mut builder = flatbuffers::FlatBufferBuilder::with_capacity(estimate_capacity(package));

    let definitions = package
        .definitions
        .iter()
        .map(|definition| encode_definition_record(&mut builder, definition))
        .collect::<Vec<_>>();
    let definitions = builder.create_vector(&definitions);
    let package_version = builder.create_string(&package.package_version);
    let source_version = builder.create_string(&package.source_version);
    let root = fb::DefinitionPackage::create(
        &mut builder,
        &fb::DefinitionPackageArgs {
            package_version: Some(package_version),
            source_version: Some(source_version),
            package_revision: package.package_revision,
            tags: package.tags,
            definitions: Some(definitions),
        },
    );
    fb::finish_definition_package_buffer(&mut builder, root);
    builder.finished_data().to_vec()
}

/// Decode verified FlatBuffers bytes into an owned definition package.
///
/// # Errors
///
/// Returns an error when the buffer does not use the expected identifier or
/// fails FlatBuffers verification.
pub fn decode_definition_package(
    bytes: &[u8],
) -> Result<DefinitionPackage, DefinitionPackageError> {
    Ok(owned_package(root_definition_package(bytes)?))
}

/// Return a verified borrowed FlatBuffers root view over a package buffer.
///
/// # Errors
///
/// Returns an error when the buffer does not use the expected identifier or
/// fails FlatBuffers verification.
pub fn root_definition_package(
    bytes: &[u8],
) -> Result<DefinitionPackageView<'_>, DefinitionPackageError> {
    if !fb::definition_package_buffer_has_identifier(bytes) {
        return Err(DefinitionPackageError::InvalidIdentifier);
    }
    Ok(fb::root_as_definition_package(bytes)?)
}

/// Returns the embedded file-definition package bundled with the DAL crate.
///
/// # Errors
///
/// Returns an error when the embedded `filedefs.dat` asset cannot be decoded.
pub fn bundled_definition_package() -> Result<&'static DefinitionPackage, DefinitionPackageError> {
    debug!(target: "dhara_storage_dal", "loading bundled definition package");
    PACKAGE
        .as_ref()
        .map_err(|message| DefinitionPackageError::BundledLoad {
            message: message.clone(),
        })
}

fn encode_definition_record<'a>(
    builder: &mut flatbuffers::FlatBufferBuilder<'a>,
    definition: &DefinitionRecord,
) -> flatbuffers::WIPOffset<fb::DefinitionRecord<'a>> {
    let file_type = builder.create_string(&definition.file_type);
    let extensions = definition
        .extensions
        .iter()
        .map(|extension| builder.create_string(extension))
        .collect::<Vec<_>>();
    let extensions = builder.create_vector(&extensions);
    let mime_type = builder.create_string(&definition.mime_type);
    let remarks = builder.create_string(&definition.remarks);
    let signature = encode_signature_definition(builder, &definition.signature);

    fb::DefinitionRecord::create(
        builder,
        &fb::DefinitionRecordArgs {
            file_type: Some(file_type),
            extensions: Some(extensions),
            mime_type: Some(mime_type),
            remarks: Some(remarks),
            signature: Some(signature),
            priority_level: definition.priority_level,
        },
    )
}

fn encode_signature_definition<'a>(
    builder: &mut flatbuffers::FlatBufferBuilder<'a>,
    signature: &SignatureDefinition,
) -> flatbuffers::WIPOffset<fb::SignatureDefinition<'a>> {
    let patterns = signature
        .patterns
        .iter()
        .map(|pattern| {
            let data = builder.create_vector(pattern.data.as_slice());
            fb::SignaturePattern::create(
                builder,
                &fb::SignaturePatternArgs {
                    position: pattern.position,
                    data: Some(data),
                },
            )
        })
        .collect::<Vec<_>>();
    let patterns = builder.create_vector(&patterns);

    let strings = signature
        .strings
        .iter()
        .map(|value| {
            let data = builder.create_vector(value.as_slice());
            fb::ByteBlob::create(builder, &fb::ByteBlobArgs { data: Some(data) })
        })
        .collect::<Vec<_>>();
    let strings = builder.create_vector(&strings);

    fb::SignatureDefinition::create(
        builder,
        &fb::SignatureDefinitionArgs {
            patterns: Some(patterns),
            strings: Some(strings),
        },
    )
}

fn owned_package(package: fb::DefinitionPackage<'_>) -> DefinitionPackage {
    DefinitionPackage {
        package_version: package.package_version().unwrap_or_default().to_owned(),
        source_version: package.source_version().unwrap_or_default().to_owned(),
        package_revision: package.package_revision(),
        tags: package.tags(),
        definitions: package
            .definitions()
            .map(|definitions| definitions.iter().map(owned_definition).collect())
            .unwrap_or_default(),
    }
}

fn owned_definition(definition: fb::DefinitionRecord<'_>) -> DefinitionRecord {
    DefinitionRecord {
        file_type: definition.file_type().unwrap_or_default().to_owned(),
        extensions: definition
            .extensions()
            .map(|extensions| extensions.iter().map(ToOwned::to_owned).collect())
            .unwrap_or_default(),
        mime_type: definition.mime_type().unwrap_or_default().to_owned(),
        remarks: definition.remarks().unwrap_or_default().to_owned(),
        signature: definition
            .signature()
            .map(owned_signature)
            .unwrap_or_default(),
        priority_level: definition.priority_level(),
    }
}

fn owned_signature(signature: fb::SignatureDefinition<'_>) -> SignatureDefinition {
    SignatureDefinition {
        patterns: signature
            .patterns()
            .map(|patterns| patterns.iter().map(owned_pattern).collect())
            .unwrap_or_default(),
        strings: signature
            .strings()
            .map(|strings| {
                strings
                    .iter()
                    .map(|value| {
                        value
                            .data()
                            .map(|data| data.iter().collect::<Vec<_>>())
                            .unwrap_or_default()
                    })
                    .collect()
            })
            .unwrap_or_default(),
    }
}

fn owned_pattern(pattern: fb::SignaturePattern<'_>) -> SignaturePattern {
    SignaturePattern {
        position: pattern.position(),
        data: pattern
            .data()
            .map(|data| data.iter().collect::<Vec<_>>())
            .unwrap_or_default(),
    }
}

fn estimate_capacity(package: &DefinitionPackage) -> usize {
    let string_bytes = package.package_version.len()
        + package.source_version.len()
        + package
            .definitions
            .iter()
            .map(|definition| {
                definition.file_type.len()
                    + definition.mime_type.len()
                    + definition.remarks.len()
                    + definition.extensions.iter().map(String::len).sum::<usize>()
            })
            .sum::<usize>();
    let byte_bytes = package
        .definitions
        .iter()
        .map(|definition| {
            definition
                .signature
                .patterns
                .iter()
                .map(|pattern| pattern.data.len())
                .sum::<usize>()
                + definition
                    .signature
                    .strings
                    .iter()
                    .map(Vec::len)
                    .sum::<usize>()
        })
        .sum::<usize>();
    (string_bytes + byte_bytes + package.definitions.len() * 128).max(1024)
}

#[cfg(test)]
mod tests {
    use super::{
        DefinitionPackage, DefinitionPackageError, DefinitionRecord, SignatureDefinition,
        SignaturePattern, bundled_definition_package, decode_definition_package,
        encode_definition_package, root_definition_package,
    };

    fn sample_package() -> DefinitionPackage {
        DefinitionPackage {
            package_version: "trid-2.00+dhbn.1".to_owned(),
            source_version: "2.00".to_owned(),
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
    fn flatbuffer_roundtrip_preserves_package_semantics() {
        let package = sample_package();
        let encoded = encode_definition_package(&package);
        let decoded = decode_definition_package(&encoded).expect("package should decode");

        assert_eq!(decoded, package);
    }

    #[test]
    fn bundled_dat_loads_successfully() {
        let package =
            bundled_definition_package().expect("bundled definitions package should decode");

        assert!(package.package_version.starts_with("trid-"));
        assert!(!package.definitions.is_empty());
    }

    #[test]
    fn root_view_reads_without_owned_decode() {
        let encoded = encode_definition_package(&sample_package());
        let root = root_definition_package(&encoded).expect("root should verify");

        assert_eq!(root.definitions().unwrap().len(), 1);
        assert_eq!(root.package_revision(), 1);
    }

    #[test]
    fn malformed_buffer_is_rejected() {
        let error = decode_definition_package(b"not-flatbuffers").unwrap_err();

        assert!(matches!(error, DefinitionPackageError::InvalidIdentifier));
    }

    #[test]
    fn wrong_identifier_is_rejected() {
        let mut builder = flatbuffers::FlatBufferBuilder::new();
        let root = builder.create_string("not a definition package");
        builder.finish(root, Some("NOPE"));
        let bytes = builder.finished_data();

        let error = decode_definition_package(bytes).unwrap_err();

        assert!(matches!(error, DefinitionPackageError::InvalidIdentifier));
    }
}
