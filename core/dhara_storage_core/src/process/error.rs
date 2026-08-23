//! Framework-level errors for task queues and process orchestration.

use thiserror::Error;

/// Errors produced by [`super::TaskQueue`] and [`super::ProcessSession`] helpers.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ProcessError {
    /// Cooperative cancellation was observed while waiting on the queue or process.
    #[error("operation cancelled while attempting to {operation}")]
    Cancelled {
        /// High-level operation label that observed cancellation.
        operation: &'static str,
    },

    /// The opposing side of the queue disconnected (producers finished or consumer dropped).
    #[error("task queue disconnected")]
    Disconnected,
}

impl ProcessError {
    /// Build a cancelled error for the given operation label.
    pub fn cancelled(operation: &'static str) -> Self {
        Self::Cancelled { operation }
    }
}
