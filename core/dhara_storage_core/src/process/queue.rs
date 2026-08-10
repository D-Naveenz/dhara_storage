//! Bounded multi-producer task queue with backpressure.

use crossbeam_channel::{Receiver, RecvTimeoutError, Sender, TryRecvError, bounded};
use std::time::Duration;

use super::cancel::StorageCancellationToken;
use super::error::ProcessError;

/// Default bounded capacity for transfer task queues.
pub const DEFAULT_TASK_QUEUE_CAPACITY: usize = 1024;

const CANCEL_POLL_INTERVAL: Duration = Duration::from_millis(25);

/// Sending half of a [`TaskQueue`].
#[derive(Debug, Clone)]
pub struct TaskQueueSender<T> {
    inner: Sender<T>,
}

/// Receiving half of a [`TaskQueue`].
#[derive(Debug)]
pub struct TaskQueueReceiver<T> {
    inner: Receiver<T>,
}

/// Bounded channel of discrete work units for a [`super::StorageProcess`].
///
/// Producers block on [`TaskQueueSender::send`] when the queue is full so a slow
/// single writer can throttle prep work (backpressure).
pub struct TaskQueue;

impl TaskQueue {
    /// Create a bounded queue with the given capacity (at least 1).
    pub fn bounded<T: Send>(capacity: usize) -> (TaskQueueSender<T>, TaskQueueReceiver<T>) {
        let capacity = capacity.max(1);
        let (sender, receiver) = bounded(capacity);
        (
            TaskQueueSender { inner: sender },
            TaskQueueReceiver { inner: receiver },
        )
    }
}

impl<T: Send> TaskQueueSender<T> {
    /// Enqueue a task, blocking while the queue is full.
    ///
    /// Polls `cancellation` while waiting so producers can stop promptly.
    ///
    /// # Errors
    ///
    /// Returns [`ProcessError::Cancelled`] when cancellation is observed, or
    /// [`ProcessError::Disconnected`] when all receivers were dropped.
    pub fn send(
        &self,
        mut item: T,
        cancellation: &StorageCancellationToken,
        operation: &'static str,
    ) -> Result<(), ProcessError> {
        loop {
            if cancellation.is_cancelled() {
                return Err(ProcessError::cancelled(operation));
            }

            match self.inner.send_timeout(item, CANCEL_POLL_INTERVAL) {
                Ok(()) => return Ok(()),
                Err(crossbeam_channel::SendTimeoutError::Timeout(returned)) => {
                    item = returned;
                }
                Err(crossbeam_channel::SendTimeoutError::Disconnected(_)) => {
                    return Err(ProcessError::Disconnected);
                }
            }
        }
    }
}

impl<T: Send> TaskQueueReceiver<T> {
    /// Receive the next task, blocking until one arrives or all senders disconnect.
    ///
    /// # Errors
    ///
    /// Returns [`ProcessError::Cancelled`] when cancellation is observed, or
    /// [`ProcessError::Disconnected`] when all senders finished and the queue is empty.
    pub fn recv(
        &self,
        cancellation: &StorageCancellationToken,
        operation: &'static str,
    ) -> Result<T, ProcessError> {
        loop {
            if cancellation.is_cancelled() {
                return Err(ProcessError::cancelled(operation));
            }

            match self.inner.recv_timeout(CANCEL_POLL_INTERVAL) {
                Ok(item) => return Ok(item),
                Err(RecvTimeoutError::Timeout) => continue,
                Err(RecvTimeoutError::Disconnected) => {
                    // Drain any last item that raced with disconnect.
                    return match self.inner.try_recv() {
                        Ok(item) => Ok(item),
                        Err(TryRecvError::Empty | TryRecvError::Disconnected) => {
                            Err(ProcessError::Disconnected)
                        }
                    };
                }
            }
        }
    }

    /// Non-blocking receive used when draining after producers join.
    pub fn try_recv(&self) -> Result<Option<T>, ProcessError> {
        match self.inner.try_recv() {
            Ok(item) => Ok(Some(item)),
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => Err(ProcessError::Disconnected),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn bounded_send_blocks_until_recv() {
        let (tx, rx) = TaskQueue::bounded::<u32>(1);
        let token = StorageCancellationToken::new();
        tx.send(1, &token, "test").unwrap();

        let token2 = token.clone();
        let handle = thread::spawn(move || {
            thread::sleep(Duration::from_millis(30));
            tx.send(2, &token2, "test")
        });

        assert_eq!(rx.recv(&token, "test").unwrap(), 1);
        assert!(handle.join().unwrap().is_ok());
        assert_eq!(rx.recv(&token, "test").unwrap(), 2);
    }

    #[test]
    fn send_observes_cancellation() {
        let (tx, _rx) = TaskQueue::bounded::<u32>(1);
        let token = StorageCancellationToken::new();
        tx.send(1, &token, "test").unwrap();
        token.cancel();
        let err = tx.send(2, &token, "test").unwrap_err();
        assert!(matches!(err, ProcessError::Cancelled { .. }));
    }
}
