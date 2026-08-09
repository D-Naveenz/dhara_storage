//! Progress reporting and cooperative cancellation for long-running storage work.
//!
//! These primitives are FS-agnostic so the runtime and future extension crates can
//! share the same transfer UX without depending on path I/O.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// Progress details emitted by long-running storage operations.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StorageProgress {
    /// Total number of bytes expected, when known in advance.
    pub total_bytes: Option<u64>,
    /// Number of bytes transferred so far.
    pub bytes_transferred: u64,
    /// Best-effort average transfer speed in bytes per second.
    pub bytes_per_second: f64,
}

/// Cooperative cancellation token for long-running storage operations.
#[derive(Debug, Clone, Default)]
pub struct StorageCancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl StorageCancellationToken {
    /// Create a new uncancelled token.
    pub fn new() -> Self {
        Self::default()
    }

    /// Mark the token as cancelled.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    /// Returns true when cancellation has been requested.
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

/// Callback interface for optional storage progress reporting.
pub trait ProgressReporter: Send + Sync + 'static {
    /// Receives a progress snapshot from an in-flight storage operation.
    fn report(&self, progress: StorageProgress);
}

impl<F> ProgressReporter for F
where
    F: Fn(StorageProgress) + Send + Sync + 'static,
{
    fn report(&self, progress: StorageProgress) {
        self(progress);
    }
}

/// Shared reporter type used by both sync and async APIs.
pub type SharedProgressReporter = Arc<dyn ProgressReporter>;
