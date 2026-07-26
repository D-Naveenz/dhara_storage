#![deny(missing_docs)]

//! Framework crate for Dhara Storage — the abstraction layer `dhara_storage` builds on.
//!
//! Today this crate ships definition-package (DSFD) primitives: schema, owned model,
//! and encode/decode. The business runtime (`dhara_storage`) embeds `filedefs.dat` and
//! owns analysis, handles, and I/O. Further framework pieces (for example process and
//! queue primitives) are planned here; they are not part of this release.

pub mod definitions;

pub use definitions::{
    DEFINITION_PACKAGE_IDENTIFIER, DEFINITION_PACKAGE_SIGNATURE, DSFD_FILE_HEADER_LEN,
    DSFD_FORMAT_VERSION, DSFD_METADATA_XMLNS, DefinitionPackage, DefinitionPackageError,
    DefinitionPackageView, DefinitionRecord, FILEDEFS_DAT_FILE_NAME, SignatureDefinition,
    SignaturePattern, decode_definition_package, encode_definition_package,
    root_definition_package,
};

/// Generated FlatBuffers accessors (stable path for tooling).
pub use definitions::generated;

/// Semver of the DSFD packaging authority (`dhara_storage_core`).
pub const PACKAGE_VERSION: &str = env!("CARGO_PKG_VERSION");
