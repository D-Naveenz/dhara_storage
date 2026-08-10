//! Long-running storage operation session: cancel, events, and task-queue helpers.

use super::cancel::StorageCancellationToken;
use super::error::ProcessError;
use super::events::StorageProcessEvent;
use super::queue::{
    DEFAULT_TASK_QUEUE_CAPACITY, TaskQueue, TaskQueueReceiver, TaskQueueSender,
};
use super::reporter::SharedProcessEventReporter;

/// Owns cancellation and the optional event sink for one storage operation.
///
/// A process is not queued — it *runs* a [`TaskQueue`] of discrete tasks (for
/// example one file each) with a single consumer to avoid destination write thrashing.
#[derive(Clone)]
pub struct StorageProcess {
    cancellation: StorageCancellationToken,
    reporter: Option<SharedProcessEventReporter>,
}

impl StorageProcess {
    /// Create a process with a fresh cancellation token and optional event reporter.
    pub fn new(
        cancellation: Option<StorageCancellationToken>,
        reporter: Option<SharedProcessEventReporter>,
    ) -> Self {
        Self {
            cancellation: cancellation.unwrap_or_default(),
            reporter,
        }
    }

    /// Shared cancellation token for producers and the consumer.
    pub fn cancellation_token(&self) -> &StorageCancellationToken {
        &self.cancellation
    }

    /// Emit a process event when a reporter is configured.
    pub fn emit(&self, event: StorageProcessEvent) {
        if let Some(reporter) = self.reporter.as_ref() {
            reporter.report(event);
        }
    }

    /// Returns true when a reporter will receive events.
    pub fn has_reporter(&self) -> bool {
        self.reporter.is_some()
    }

    /// Open a bounded task queue using the default capacity.
    pub fn task_queue<T: Send>(&self) -> (TaskQueueSender<T>, TaskQueueReceiver<T>) {
        TaskQueue::bounded(DEFAULT_TASK_QUEUE_CAPACITY)
    }

    /// Open a bounded task queue with an explicit capacity.
    pub fn task_queue_with_capacity<T: Send>(
        &self,
        capacity: usize,
    ) -> (TaskQueueSender<T>, TaskQueueReceiver<T>) {
        TaskQueue::bounded(capacity)
    }

    /// Fail fast if cancellation was requested.
    ///
    /// # Errors
    ///
    /// Returns [`ProcessError::Cancelled`] when the token is set.
    pub fn ensure_not_cancelled(&self, operation: &'static str) -> Result<(), ProcessError> {
        if self.cancellation.is_cancelled() {
            return Err(ProcessError::cancelled(operation));
        }
        Ok(())
    }

    /// Consume tasks until producers disconnect, invoking `consume` for each item.
    ///
    /// [`ProcessError::Disconnected`] from the receiver means producers finished and
    /// is treated as success after the queue drains.
    ///
    /// # Errors
    ///
    /// Propagates consumer errors and cancellation.
    pub fn consume_tasks<T, E>(
        &self,
        receiver: TaskQueueReceiver<T>,
        operation: &'static str,
        mut consume: impl FnMut(T) -> Result<(), E>,
        map_process_error: impl Fn(ProcessError) -> E,
    ) -> Result<(), E>
    where
        T: Send,
    {
        loop {
            match receiver.recv(&self.cancellation, operation) {
                Ok(task) => consume(task)?,
                Err(ProcessError::Disconnected) => return Ok(()),
                Err(err) => return Err(map_process_error(err)),
            }
        }
    }
}
