//! Free functions for file and directory create/copy/move/delete with progress options.
//!
//! Prefer [`crate::storage`] handles for a higher-level API; this module is the
//! sync-first implementation surface (plus optional Tokio wrappers). Progress,
//! cancellation, and option types are defined in `dhara_storage_core` and
//! re-exported here. Copy/move return [`crate::process::StorageProcess`] via
//! `start_*` helpers; sync sugar waits inside.

pub(crate) mod common;
mod directory;
pub(crate) mod file;
pub mod open;
pub(crate) mod transfer;

#[cfg(feature = "async-tokio")]
mod tokio;

pub use crate::process::{ProcessOutcome, StorageProcess};
pub use dhara_storage_core::{
    DirectoryDeleteOptions, FileShareMode, OpenReadOptions, OpenWriteOptions, ProcessEventReporter,
    ReadOptions, SharedProcessEventReporter, StorageCancellationToken, StorageProcessEvent,
    TransferOptions, WriteOptions,
};
pub use open::{open_for_read, open_for_write};
pub use directory::{
    copy_directory, copy_directory_with_options, create_directory, create_directory_all,
    delete_directory, delete_directory_with_options, execute_copy_directory_with_options,
    execute_move_directory_with_options, move_directory, move_directory_with_options,
    rename_directory, start_copy_directory, start_copy_directory_with_options,
    start_move_directory, start_move_directory_with_options,
};
pub use file::{
    copy_file, copy_file_with_options, delete_file, execute_copy_file_with_options,
    execute_move_file_with_options, move_file, move_file_with_options, read_file,
    read_file_to_string, rename_file, start_copy_file, start_copy_file_with_options,
    start_move_file, start_move_file_with_options, write_file, write_file_from_reader,
    write_file_string,
};

#[cfg(feature = "async-tokio")]
pub use tokio::{
    create_directory_all_async, create_directory_async, delete_directory_async, delete_file_async,
    read_file_async, read_file_to_string_async, rename_directory_async, rename_file_async,
    write_file_async, write_file_from_reader_async, write_file_string_async,
};
