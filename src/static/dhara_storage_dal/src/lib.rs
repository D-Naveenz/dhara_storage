#![deny(missing_docs)]

//! FlatBuffers data access layer for Dhara Storage file definitions.
//!
//! This crate owns the internal `filedefs.dat` artifact format and provides
//! owned model types plus serializer/deserializer helpers for the runtime and
//! repository tooling crates.

mod bundled;
mod codec;
mod container;
mod error;
mod metadata_xml;
mod model;

/// Generated FlatBuffers accessors.
pub mod generated {
    #![allow(clippy::missing_safety_doc)]
    #![allow(missing_docs)]
    include!("generated/filedefs_generated.rs");
}

pub use bundled::bundled_definition_package;
pub use container::{
    decode_definition_package, encode_definition_package, root_definition_package,
};
pub use error::DefinitionPackageError;
pub use model::{
    DEFINITION_PACKAGE_IDENTIFIER, DEFINITION_PACKAGE_SIGNATURE, DSFD_END_MAGIC_LEN,
    DSFD_FILE_HEADER_LEN, DSFD_FORMAT_VERSION, DSFD_METADATA_XMLNS, DefinitionPackage,
    DefinitionPackageView, DefinitionRecord, FILEDEFS_DAT_FILE_NAME, SignatureDefinition,
    SignaturePattern,
};
