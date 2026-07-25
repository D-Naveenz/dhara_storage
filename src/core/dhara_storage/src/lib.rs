//! Rust-native storage and content-analysis primitives for Dhara Storage.
//!
//! This crate provides definition-driven file typing (bundled `filedefs.dat`),
//! path-based file and directory handles, transfers with optional progress, and
//! debounced directory watching. The definition package is loaded through
//! `dhara_storage_dal`.

#![deny(missing_docs)]

/// Content-based file analysis and heuristic classification helpers.
pub mod analysis;
/// Runtime file-definition package types and decoders.
pub mod definitions;
/// Typed error values returned by storage operations and metadata queries.
pub mod error;
/// Immutable file-system metadata models and formatting helpers.
pub mod info;
/// File and directory mutation APIs plus progress and cancellation primitives.
pub mod operations;
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
pub use info::{
    DEFAULT_SHELL_ICON_SIZE, DirectoryInfo, DirectorySummary, FileInfo, ShellDetails, ShellIcon,
    SizeUnit, StorageMetadata, format_size,
};
pub use operations::{
    DirectoryDeleteOptions, ProgressReporter, ReadOptions, SharedProgressReporter,
    StorageCancellationToken, StorageProgress, TransferOptions, WriteOptions, copy_directory,
    copy_directory_with_options, copy_file, copy_file_with_options, create_directory,
    create_directory_all, delete_directory, delete_directory_with_options, delete_file,
    move_directory, move_directory_with_options, move_file, move_file_with_options, read_file,
    read_file_to_string, rename_directory, rename_file, write_file, write_file_from_reader,
    write_file_string,
};
#[cfg(feature = "async-tokio")]
pub use operations::{
    copy_directory_async, copy_file_async, create_directory_all_async, create_directory_async,
    delete_directory_async, delete_file_async, move_directory_async, move_file_async,
    read_file_async, read_file_to_string_async, rename_directory_async, rename_file_async,
    write_file_async, write_file_from_reader_async, write_file_string_async,
};
pub use storage::{DirectoryStorage, FileStorage, SearchScope, StorageEntry};
pub use watch::{DirectoryWatchHandle, StorageChangeEvent, StorageChangeType, StorageWatchConfig};
