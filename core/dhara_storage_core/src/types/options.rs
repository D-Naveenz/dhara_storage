//! Option bundles for transfer, read, write, and directory-delete operations.

use crate::process::{SharedProcessEventReporter, StorageCancellationToken};

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
