#![deny(missing_docs)]

//! Framework crate for Dhara Storage — the abstraction layer `dhara_storage` builds on.
//!
//! This crate ships reusable framework primitives:
//! - **definitions** — DSFD schema, owned model, and encode/decode
//! - **process** — cancellation, task queues, process sessions, and event streams
//! - **types** — operation options and portable metadata value shapes
//!
//! The business runtime (`dhara_storage`) embeds `filedefs.dat` and owns analysis,
//! path-based handles, filesystem I/O, watching, and shell metadata. Extension
//! crates may depend on the runtime and compose its concrete handles.

/// File-definition package support (DSFD).
pub mod definitions;
/// Progress events, task queues, and cooperative cancellation.
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
    BytesEventThrottle, DEFAULT_BYTES_EVENT_DELTA, DEFAULT_BYTES_EVENT_INTERVAL,
    DEFAULT_TASK_QUEUE_CAPACITY, ProcessError, ProcessEventReporter, ProcessSession,
    SharedProcessEventReporter, StorageCancellationToken, StorageProcessEvent, TaskQueue,
    TaskQueueReceiver, TaskQueueSender,
};
pub use types::{
    DirectoryDeleteOptions, ReadOptions, SizeUnit, StorageAttributes, StoragePermissions,
    StorageSize, TransferOptions, WriteOptions, format_size,
};

/// Generated FlatBuffers accessors (stable path for tooling).
pub use definitions::generated;

/// Semver of the DSFD packaging authority (`dhara_storage_core`).
pub const PACKAGE_VERSION: &str = env!("CARGO_PKG_VERSION");
