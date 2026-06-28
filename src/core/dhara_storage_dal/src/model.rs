use crate::generated::dhara::storage::dal as fb;

/// Human-readable signature stored in the XML metadata footer.
pub const DEFINITION_PACKAGE_SIGNATURE: &str = "Dhara Storage File Definition package - DSFD";

/// Four-byte file magic at the start of `filedefs.dat`.
pub const DEFINITION_PACKAGE_IDENTIFIER: &str = "DSFD";

// SCHEMA_URL — replace branch/tag if the canonical path changes.
// This URL resolves only after the XSD is committed on the default branch.
// Local builds validate against the checked-in file; the URL is for consumers.
/// XML namespace for DSFD metadata documents.
pub const DSFD_METADATA_XMLNS: &str = "https://raw.githubusercontent.com/D-Naveenz/dhara_storage/main/src/core/dhara_storage_dal/schema/dsfd-metadata.xsd";

/// Current on-disk container format version.
pub const DSFD_FORMAT_VERSION: u16 = 2;

/// Byte length of the fixed file header at the start of `filedefs.dat`.
pub const DSFD_FILE_HEADER_LEN: usize = 10;

/// Default file name for embedded file-definition packages.
pub const FILEDEFS_DAT_FILE_NAME: &str = "filedefs.dat";

/// Borrowed FlatBuffers root view over a definition package payload.
pub type DefinitionPackageView<'a> = fb::DefinitionPackage<'a>;

/// Serialized file-definition package loaded from `filedefs.dat`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefinitionPackage {
    /// Version of `dhara_tool` used to build the package.
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
