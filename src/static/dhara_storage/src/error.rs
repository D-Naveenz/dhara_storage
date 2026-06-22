use std::io;
use std::path::PathBuf;

use thiserror::Error;

/// Errors produced by Dhara storage analysis and metadata operations.
#[derive(Debug, Error)]
pub enum StorageError {
    /// The supplied path did not exist at the time of access.
    #[error("path does not exist: {path}")]
    NotFound {
        /// The absolute or caller-supplied path that could not be resolved.
        path: PathBuf,
    },

    /// The destination path already existed and overwrite was disabled.
    #[error("path already exists: {path}")]
    AlreadyExists {
        /// The destination path that prevented the operation from continuing.
        path: PathBuf,
    },

    /// The supplied path existed but did not refer to a file.
    #[error("path is not a file: {path}")]
    NotAFile {
        /// The path that failed the file-only contract.
        path: PathBuf,
    },

    /// The supplied path existed but did not refer to a directory.
    #[error("path is not a directory: {path}")]
    NotADirectory {
        /// The path that failed the directory-only contract.
        path: PathBuf,
    },

    /// A file-system name argument was invalid for the requested operation.
    #[error("invalid {kind} name: {value}")]
    InvalidName {
        /// A short label describing the name slot that failed validation.
        kind: &'static str,
        /// The raw value supplied by the caller.
        value: String,
    },

    /// The requested operation could not be completed because the path shape conflicted with the API contract.
    #[error("path conflict at '{path}': {message}")]
    PathConflict {
        /// The path whose file-versus-directory shape conflicted with the requested operation.
        path: PathBuf,
        /// A stable description of the contract violation.
        message: &'static str,
    },

    /// A path-based I/O operation failed.
    #[error("failed to {operation} '{path}': {source}")]
    Io {
        /// The high-level operation that was in progress when the error occurred.
        operation: &'static str,
        /// The path associated with the failed filesystem call.
        path: PathBuf,
        /// The underlying operating-system or standard-library I/O error.
        #[source]
        source: io::Error,
    },

    /// A reader-based I/O operation failed.
    #[error("failed to {operation} reader: {source}")]
    ReaderIo {
        /// The high-level read or write phase that failed.
        operation: &'static str,
        /// The underlying reader or writer error.
        #[source]
        source: io::Error,
    },

    /// The embedded definitions package could not be decoded.
    #[error("failed to load embedded file definitions: {message}")]
    DefinitionsLoad {
        /// The decode or validation failure message returned by the package loader.
        message: String,
    },

    /// The requested operation was cancelled before completion.
    #[error("operation cancelled while attempting to {operation}")]
    Cancelled {
        /// The high-level operation that observed cancellation.
        operation: &'static str,
    },

    /// An async runtime task failed before the storage operation completed.
    #[error("async runtime failed to {operation}: {message}")]
    AsyncRuntime {
        /// The high-level operation that depended on the async runtime.
        operation: &'static str,
        /// The runtime failure details surfaced by the async wrapper.
        message: String,
    },

    /// A storage watcher failed to initialize or deliver events.
    #[error("watch error while attempting to {operation}: {message}")]
    Watch {
        /// The watch lifecycle phase that failed.
        operation: &'static str,
        /// The watcher or debouncer failure details.
        message: String,
    },
}

impl StorageError {
    pub(crate) fn io(operation: &'static str, path: impl Into<PathBuf>, source: io::Error) -> Self {
        Self::Io {
            operation,
            path: path.into(),
            source,
        }
    }

    pub(crate) fn reader_io(operation: &'static str, source: io::Error) -> Self {
        Self::ReaderIo { operation, source }
    }

    pub(crate) fn already_exists(path: impl Into<PathBuf>) -> Self {
        Self::AlreadyExists { path: path.into() }
    }

    pub(crate) fn invalid_name(kind: &'static str, value: impl Into<String>) -> Self {
        Self::InvalidName {
            kind,
            value: value.into(),
        }
    }

    pub(crate) fn path_conflict(path: impl Into<PathBuf>, message: &'static str) -> Self {
        Self::PathConflict {
            path: path.into(),
            message,
        }
    }

    pub(crate) fn cancelled(operation: &'static str) -> Self {
        Self::Cancelled { operation }
    }

    #[cfg(feature = "async-tokio")]
    pub(crate) fn async_runtime(operation: &'static str, message: impl Into<String>) -> Self {
        Self::AsyncRuntime {
            operation,
            message: message.into(),
        }
    }

    pub(crate) fn watch(operation: &'static str, message: impl Into<String>) -> Self {
        Self::Watch {
            operation,
            message: message.into(),
        }
    }
}
