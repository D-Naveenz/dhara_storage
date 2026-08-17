//! Rust-native storage and content-analysis primitives for Dhara Storage.
//!
//! This crate provides definition-driven file typing (bundled `filedefs.dat`),
//! path-based file and directory handles, transfers with optional progress, and
//! debounced directory watching. Framework primitives (DSFD encode/decode,
//! progress/cancel, portable value types) live in `dhara_storage_core` and are
//! re-exported here for the usual application dependency path. This crate embeds
//! and indexes the runtime `filedefs.dat`.

#![deny(missing_docs)]

/// Content-based file analysis and heuristic classification helpers.
pub mod analysis;
/// Runtime file-definition package types and decoders.
pub mod definitions;
/// Typed error values returned by storage operations and metadata queries.
pub mod error;
/// On-demand metadata, attributes, permissions, and size helpers.
pub mod metadata;
/// File and directory mutation APIs (progress/cancel types from core, re-exported).
pub mod operations;
/// Awaitable storage process handle for in-flight copy/move work.
pub mod process;
/// Path-based storage handles layered over the core operation APIs.
pub mod storage;
/// Debounced directory watching primitives.
pub mod watch;

pub use analysis::{AnalysisReport, ContentKind, DetectedDefinition, analyze_path, analyze_reader};
pub use definitions::{
    DEFINITION_PACKAGE_ID, DefinitionPackage, DefinitionPackageDecodeError, DefinitionRecord,
    SignatureDefinition, SignaturePattern, bundled_definition_package, decode_definition_package,
};
pub use error::StorageError;
pub use metadata::{
    DEFAULT_SHELL_ICON_SIZE, DirectoryMetadata, DirectorySummary, FileExtension, FileMetadata,
    ShellIcon, SizeUnit, StorageAttributes, StorageMetadata, StoragePermissions, StorageSize,
    StorageType, apply_storage_attributes, format_size, is_temporary_path, scan_directory_summary,
};
pub use operations::{
    DirectoryDeleteOptions, FileShareMode, OpenReadOptions, OpenWriteOptions, ProcessEventReporter,
    ProcessOutcome, ReadOptions, SharedProcessEventReporter, StorageCancellationToken,
    StorageProcess, StorageProcessEvent, TransferOptions, WriteOptions, copy_directory,
    copy_directory_with_options, copy_file, copy_file_with_options, create_directory,
    create_directory_all, delete_directory, delete_directory_with_options, delete_file,
    execute_copy_directory_with_options, execute_copy_file_with_options,
    execute_move_directory_with_options, execute_move_file_with_options, move_directory,
    move_directory_with_options, move_file, move_file_with_options, open_for_read, open_for_write,
    read_file, read_file_to_string, rename_directory, rename_file, start_copy_directory,
    start_copy_directory_with_options, start_copy_file, start_copy_file_with_options,
    start_move_directory, start_move_directory_with_options, start_move_file,
    start_move_file_with_options, write_file, write_file_from_reader, write_file_string,
};
#[cfg(feature = "async-tokio")]
pub use operations::{
    create_directory_all_async, create_directory_async, delete_directory_async, delete_file_async,
    read_file_async, read_file_to_string_async, rename_directory_async, rename_file_async,
    write_file_async, write_file_from_reader_async, write_file_string_async,
};
pub use storage::{DirectoryStorage, FileStorage, SearchScope, StorageEntry};
pub use watch::{DirectoryWatchHandle, StorageChangeEvent, StorageChangeType, StorageWatchConfig};
