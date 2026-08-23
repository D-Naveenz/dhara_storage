//! File-definition package support (DSFD) — framework definitions slice.
//!
//! Owns the on-disk package format, FlatBuffers schema, owned model types, and
//! encode/decode helpers. The compile-time embedded `filedefs.dat` asset lives
//! in `dhara_storage`, not here.

mod error;
mod format;
mod model;

/// Generated FlatBuffers accessors for definition packages.
pub mod generated {
    #![allow(clippy::missing_safety_doc)]
    #![allow(missing_docs)]
    include!("generated/filedefs_generated.rs");
}

pub use error::DefinitionPackageError;
pub use format::{decode_definition_package, encode_definition_package, root_definition_package};
pub use model::{
    DEFINITION_PACKAGE_IDENTIFIER, DEFINITION_PACKAGE_SIGNATURE, DSFD_FILE_HEADER_LEN,
    DSFD_FORMAT_VERSION, DSFD_METADATA_XMLNS, DefinitionPackage, DefinitionPackageView,
    DefinitionRecord, FILEDEFS_DAT_FILE_NAME, SignatureDefinition, SignaturePattern,
};
