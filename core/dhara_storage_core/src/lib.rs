#![deny(missing_docs)]

//! Framework crate for Dhara Storage — the abstraction layer `dhara_storage` builds on.
//!
//! This crate ships reusable framework primitives:
//! - **definitions** — DSFD schema, owned model, and encode/decode
//! - **process** — progress reporting and cooperative cancellation
//! - **types** — operation options and portable metadata value shapes
//!
//! The business runtime (`dhara_storage`) embeds `filedefs.dat` and owns analysis,
//! path-based handles, filesystem I/O, watching, and shell metadata. Extension
//! crates may depend on the runtime and compose its concrete handles.
//!
//! Higher-level process orchestration (`StorageProcess`, `ProcessingQueue`) is
//! planned as a later core slice on top of [`process`].

/// File-definition package support (DSFD).
pub mod definitions;
/// Progress and cancellation primitives for long-running work.
pub mod process;
/// Portable value types and operation option bundles.
pub mod types;

pub use definitions::{
    DEFINITION_PACKAGE_IDENTIFIER, DEFINITION_PACKAGE_SIGNATURE, DSFD_FILE_HEADER_LEN,
    DSFD_FORMAT_VERSION, DSFD_METADATA_XMLNS, DefinitionPackage, DefinitionPackageError,
    DefinitionPackageView, DefinitionRecord, FILEDEFS_DAT_FILE_NAME, SignatureDefinition,
    SignaturePattern, decode_definition_package, encode_definition_package,
    root_definition_package,
};
pub use process::{
    ProgressReporter, SharedProgressReporter, StorageCancellationToken, StorageProgress,
};
pub use types::{
    DirectoryDeleteOptions, ReadOptions, SizeUnit, StorageAttributes, StoragePermissions,
    StorageSize, TransferOptions, WriteOptions, format_size,
};

/// Generated FlatBuffers accessors (stable path for tooling).
pub use definitions::generated;

/// Semver of the DSFD packaging authority (`dhara_storage_core`).
pub const PACKAGE_VERSION: &str = env!("CARGO_PKG_VERSION");
