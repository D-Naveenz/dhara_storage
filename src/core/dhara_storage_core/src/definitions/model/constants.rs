//! DSFD format constants shared by encode/decode and tooling.

/// Human-readable signature stored in the XML metadata footer.
pub const DEFINITION_PACKAGE_SIGNATURE: &str = "Dhara Storage File Definition package - DSFD";

/// Four-byte file magic at the start of `filedefs.dat`.
pub const DEFINITION_PACKAGE_IDENTIFIER: &str = "DSFD";

// SCHEMA_URL — replace branch/tag if the canonical path changes.
// This URL resolves only after the XSD is committed on the default branch.
// Local builds validate against the checked-in file; the URL is for consumers.
/// XML namespace for DSFD metadata documents.
pub const DSFD_METADATA_XMLNS: &str = "https://raw.githubusercontent.com/D-Naveenz/dhara_storage/main/src/core/dhara_storage_core/schema/dsfd-metadata.xsd";

/// Legacy xmlns written by `dhara_storage_dal` packages; accepted on decode only.
pub(crate) const DSFD_METADATA_XMLNS_LEGACY: &str = "https://raw.githubusercontent.com/D-Naveenz/dhara_storage/main/src/core/dhara_storage_dal/schema/dsfd-metadata.xsd";

/// Current on-disk container format version.
pub const DSFD_FORMAT_VERSION: u16 = 2;

/// Byte length of the fixed file header at the start of `filedefs.dat`.
pub const DSFD_FILE_HEADER_LEN: usize = 10;

/// Default file name for file-definition packages.
pub const FILEDEFS_DAT_FILE_NAME: &str = "filedefs.dat";
