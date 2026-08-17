//! Public awaitable [`StorageProcess`] handle for in-flight storage work.
//!
//! Engine sessions ([`dhara_storage_core::ProcessSession`]) stay private to the
//! transfer pipeline. Callers wait or await this handle for terminal completion.

use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};
use std::thread::{self, JoinHandle};

use dhara_storage_core::StorageCancellationToken;

use crate::error::StorageError;

/// Terminal result of a finished [`StorageProcess`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessOutcome {
    /// Destination path when the operation produced one (copy/move).
    pub destination: Option<PathBuf>,
}

struct SharedState {
    result: Option<Result<ProcessOutcome, StorageError>>,
    waker: Option<Waker>,
}

/// Running storage operation: cancel, wait, or await until terminal completion.
///
/// Work starts as soon as the process is created. Sync APIs call [`Self::wait`];
/// async callers can `await` the same handle (`Future` + `GetAwaiter`-shaped model on .NET).
pub struct StorageProcess {
    cancellation: StorageCancellationToken,
    shared: Arc<Mutex<SharedState>>,
    join: Option<JoinHandle<()>>,
}

impl StorageProcess {
    /// Spawn `work` on a dedicated thread and return a handle immediately.
    pub(crate) fn spawn<F>(cancellation: StorageCancellationToken, work: F) -> Self
    where
        F: FnOnce() -> Result<ProcessOutcome, StorageError> + Send + 'static,
    {
        let shared = Arc::new(Mutex::new(SharedState {
            result: None,
            waker: None,
        }));
        let shared_thread = Arc::clone(&shared);
        let join = thread::Builder::new()
            .name("dhara-storage-process".into())
            .spawn(move || {
                let result = work();
                let mut guard = shared_thread
                    .lock()
                    .expect("storage process shared state poisoned");
                guard.result = Some(result);
                if let Some(waker) = guard.waker.take() {
                    waker.wake();
                }
            })
            .expect("failed to spawn storage process thread");

        Self {
            cancellation,
            shared,
            join: Some(join),
        }
    }

    /// Shared cancellation token for this process (and its transfer session).
    pub fn cancellation_token(&self) -> &StorageCancellationToken {
        &self.cancellation
    }

    /// Block until the process reaches a terminal state.
    ///
    /// # Errors
    ///
    /// Returns the engine error, including cancellation.
    pub fn wait(mut self) -> Result<ProcessOutcome, StorageError> {
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
        let mut guard = self
            .shared
            .lock()
            .expect("storage process shared state poisoned");
        guard.result.take().unwrap_or_else(|| {
            Err(StorageError::path_conflict(
                PathBuf::new(),
                "storage process ended without a result",
            ))
        })
    }

    /// Block until completion and discard the outcome (sync copy/move sugar).
    ///
    /// # Errors
    ///
    /// Returns the engine error, including cancellation.
    pub fn wait_unit(self) -> Result<(), StorageError> {
        self.wait().map(|_| ())
    }
}

impl Future for StorageProcess {
    type Output = Result<ProcessOutcome, StorageError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        let mut guard = this
            .shared
            .lock()
            .expect("storage process shared state poisoned");
        if let Some(result) = guard.result.take() {
            drop(guard);
            if let Some(join) = this.join.take() {
                let _ = join.join();
            }
            Poll::Ready(result)
        } else {
            guard.waker = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}
