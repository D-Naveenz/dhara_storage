//! Process event stream for long-running storage operations.
//!
//! Events follow a Windows-copy-dialog shape: session totals once, current item
//! when the active file changes, and lean byte ticks (hosts derive speed/ETA).

use std::path::PathBuf;

/// Event emitted by a process session / public storage process during long-running work.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StorageProcessEvent {
    /// Planning finished; totals are known and transfer has not started writing.
    Started {
        /// Aggregate byte size of the planned work.
        total_bytes: u64,
        /// Number of file tasks in the planned work.
        total_files: u64,
    },
    /// The writer began a new file task.
    CurrentItem {
        /// Path shown to hosts (typically the destination being written).
        path: PathBuf,
        /// Size of the current file in bytes.
        file_size: u64,
        /// Zero-based index of this file within the process.
        file_index: u64,
    },
    /// Cumulative bytes transferred so far for the whole process.
    ///
    /// Does not repeat totals or speed — hosts compute rate from wall time.
    Bytes {
        /// Bytes transferred across all tasks so far.
        bytes_transferred: u64,
    },
    /// The process finished successfully.
    Completed {
        /// Primary destination path when applicable.
        destination: Option<PathBuf>,
    },
    /// The process failed after starting.
    Failed {
        /// Stable, display-oriented failure message.
        message: String,
    },
    /// Cooperative cancellation was observed.
    Cancelled {
        /// High-level operation label that observed cancellation.
        operation: &'static str,
    },
}
