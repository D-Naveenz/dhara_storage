//! Option bundles for transfer, read, write, open, and directory-delete operations.

use std::time::Duration;

use crate::process::{SharedProcessEventReporter, StorageCancellationToken};

/// How other processes may share an opened file (Windows share mode; Unix ignores exclusivity nuances where N/A).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FileShareMode {
    /// Allow other readers/writers/deleters where the OS supports it (default for read opens).
    #[default]
    Shared,
    /// Deny sharing so the opener holds exclusive access (default for write opens).
    Exclusive,
}

/// Options for engine/interop read opens.
///
/// Not a product-facing storage-handle API; used by the runtime and `dhara-sd`.
#[derive(Debug, Clone)]
pub struct OpenReadOptions {
    /// Share mode applied when opening for read.
    pub share: FileShareMode,
    /// When set, retry on sharing/busy errors until the deadline; `None` fails immediately.
    pub lock_timeout: Option<Duration>,
}

impl Default for OpenReadOptions {
    fn default() -> Self {
        Self {
            share: FileShareMode::Shared,
            lock_timeout: None,
        }
    }
}

/// Options for engine/interop write opens.
///
/// Not a product-facing storage-handle API; used by the runtime and `dhara-sd`.
#[derive(Debug, Clone)]
pub struct OpenWriteOptions {
    /// Share mode applied when opening for write.
    pub share: FileShareMode,
    /// Replace an existing file (`truncate`) when true; otherwise `create_new`.
    pub overwrite: bool,
    /// Create missing parent directories before opening.
    pub create_parent_directories: bool,
    /// When set, retry on sharing/busy errors until the deadline; `None` fails immediately.
    pub lock_timeout: Option<Duration>,
}

impl Default for OpenWriteOptions {
    fn default() -> Self {
        Self {
            share: FileShareMode::Exclusive,
            overwrite: true,
            create_parent_directories: false,
            lock_timeout: None,
        }
    }
}

/// Common options for copy and move style operations.
#[derive(Clone, Default)]
pub struct TransferOptions {
    /// Replace the destination when it already exists.
    pub overwrite: bool,
    /// Override the buffered copy size when progress reporting is enabled.
    pub buffer_size: Option<usize>,
    /// Optional process event sink (Windows-copy-dialog style stream).
    pub progress: Option<SharedProcessEventReporter>,
    /// Optional cancellation token for cooperative cancellation.
    pub cancellation_token: Option<StorageCancellationToken>,
}

/// Common options for byte-oriented read operations.
#[derive(Clone, Default)]
pub struct ReadOptions {
    /// Override the buffered read size when progress reporting is enabled.
    pub buffer_size: Option<usize>,
    /// Optional process event sink for long reads.
    pub progress: Option<SharedProcessEventReporter>,
    /// Optional cancellation token for cooperative cancellation.
    pub cancellation_token: Option<StorageCancellationToken>,
}

/// Common options for file write operations.
#[derive(Clone)]
pub struct WriteOptions {
    /// Replace the destination when it already exists.
    pub overwrite: bool,
    /// Create missing parent directories before opening the destination.
    pub create_parent_directories: bool,
    /// Override the buffered copy size when progress reporting is enabled.
    pub buffer_size: Option<usize>,
    /// Optional process event sink for long writes.
    pub progress: Option<SharedProcessEventReporter>,
    /// Optional cancellation token for cooperative cancellation.
    pub cancellation_token: Option<StorageCancellationToken>,
}

impl Default for WriteOptions {
    fn default() -> Self {
        Self {
            overwrite: true,
            create_parent_directories: true,
            buffer_size: None,
            progress: None,
            cancellation_token: None,
        }
    }
}

/// Options for directory deletion.
#[derive(Debug, Clone)]
pub struct DirectoryDeleteOptions {
    /// Delete the directory recursively when true.
    pub recursive: bool,
    /// Optional cancellation token for cooperative cancellation.
    pub cancellation_token: Option<StorageCancellationToken>,
}

impl Default for DirectoryDeleteOptions {
    fn default() -> Self {
        Self {
            recursive: true,
            cancellation_token: None,
        }
    }
}
