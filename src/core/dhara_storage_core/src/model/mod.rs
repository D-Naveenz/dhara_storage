//! Domain model for definition packages, records, and signatures.

mod constants;

use crate::generated::dhara::storage::core as fb;

pub(crate) use constants::DSFD_METADATA_XMLNS_LEGACY;
pub use constants::{
    DEFINITION_PACKAGE_IDENTIFIER, DEFINITION_PACKAGE_SIGNATURE, DSFD_FILE_HEADER_LEN,
    DSFD_FORMAT_VERSION, DSFD_METADATA_XMLNS, FILEDEFS_DAT_FILE_NAME,
};

/// Borrowed FlatBuffers root view over a definition package payload.
pub type DefinitionPackageView<'a> = fb::DefinitionPackage<'a>;

/// Serialized file-definition package loaded from `filedefs.dat`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefinitionPackage {
    /// Version of the DSFD packaging authority used to build the package.
    pub package_version: String,
    /// ISO `YYYY-MM-DD` release date of the upstream TrID definitions dataset.
    pub definitions_release: String,
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
