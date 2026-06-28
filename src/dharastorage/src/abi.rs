use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use dhara_storage::{
    DirectoryDeleteOptions, StorageCancellationToken, StorageProgress, TransferOptions,
    WriteOptions,
};

use crate::errors::{ErrorPayload, FfiFailure};

/// Status code returned by every exported Dhara Storage FFI function.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DharaStatus {
    /// The call completed successfully.
    Ok = 0,
    /// The call failed and an error payload may be available in the provided output buffer.
    Error = 1,
    /// The caller supplied an invalid pointer, flag, or argument value.
    InvalidArgument = 2,
    /// The native call panicked before it could produce a normal status code.
    Panic = 3,
}

/// Current lifecycle state of a background native operation handle.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DharaOperationState {
    /// The worker thread is still processing the requested operation.
    Running = 0,
    /// The operation completed successfully and a result may be available.
    Completed = 1,
    /// The operation terminated with an error payload.
    Failed = 2,
    /// The operation observed cooperative cancellation before completion.
    Cancelled = 3,
}

/// Snapshot of the latest background-operation progress reported through the FFI.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DharaOperationSnapshot {
    /// Current lifecycle state of the background operation.
    pub state: DharaOperationState,
    /// Non-zero when `total_bytes` contains a meaningful expected byte count.
    pub has_total_bytes: u8,
    /// Expected total byte count when `has_total_bytes` is non-zero.
    pub total_bytes: u64,
    /// Number of bytes transferred so far.
    pub bytes_transferred: u64,
    /// Best-effort average transfer rate in bytes per second.
    pub bytes_per_second: f64,
}

#[derive(Debug, Clone)]
pub(crate) struct SharedProgressState {
    pub(crate) total_bytes: Option<u64>,
    pub(crate) bytes_transferred: u64,
    pub(crate) bytes_per_second: f64,
}

#[derive(Debug)]
pub(crate) enum OperationResult {
    None,
    String(String),
    Bytes(Vec<u8>),
}

#[derive(Debug)]
pub struct NativeOperationHandle {
    pub(crate) state: AtomicU8,
    pub(crate) progress: Mutex<SharedProgressState>,
    pub(crate) result: Mutex<Option<OperationResult>>,
    pub(crate) error: Mutex<Option<FfiFailure>>,
    pub(crate) cancellation_token: StorageCancellationToken,
    pub(crate) worker: Mutex<Option<JoinHandle<()>>>,
}

pub(crate) struct NativeProgressReporter {
    pub(crate) state: Arc<NativeOperationHandle>,
}

pub struct NativeWatchHandle {
    pub(crate) handle: Mutex<Option<dhara_storage::DirectoryWatchHandle>>,
}

pub struct NativeWriteSession {
    pub(crate) path: PathBuf,
    pub(crate) file: Mutex<Option<File>>,
    pub(crate) completed: AtomicBool,
}

impl dhara_storage::ProgressReporter for NativeProgressReporter {
    fn report(&self, progress: StorageProgress) {
        if let Ok(mut state) = self.state.progress.lock() {
            state.total_bytes = progress.total_bytes;
            state.bytes_transferred = progress.bytes_transferred;
            state.bytes_per_second = progress.bytes_per_second;
        }
    }
}

impl NativeOperationHandle {
    pub(crate) fn new() -> Arc<Self> {
        Arc::new(Self {
            state: AtomicU8::new(DharaOperationState::Running as u8),
            progress: Mutex::new(SharedProgressState {
                total_bytes: None,
                bytes_transferred: 0,
                bytes_per_second: 0.0,
            }),
            result: Mutex::new(None),
            error: Mutex::new(None),
            cancellation_token: StorageCancellationToken::new(),
            worker: Mutex::new(None),
        })
    }

    pub(crate) fn state(&self) -> DharaOperationState {
        match self.state.load(Ordering::SeqCst) {
            0 => DharaOperationState::Running,
            1 => DharaOperationState::Completed,
            2 => DharaOperationState::Failed,
            3 => DharaOperationState::Cancelled,
            _ => DharaOperationState::Failed,
        }
    }

    pub(crate) fn set_completed(&self, result: OperationResult) {
        if let Ok(mut slot) = self.result.lock() {
            *slot = Some(result);
        }
        self.state
            .store(DharaOperationState::Completed as u8, Ordering::SeqCst);
    }

    pub(crate) fn set_failure(&self, failure: FfiFailure) {
        let state = if failure.payload.code == "cancelled" {
            DharaOperationState::Cancelled
        } else {
            DharaOperationState::Failed
        };
        if let Ok(mut slot) = self.error.lock() {
            *slot = Some(failure);
        }
        self.state.store(state as u8, Ordering::SeqCst);
    }

    pub(crate) fn snapshot(&self) -> DharaOperationSnapshot {
        let progress = self.progress.lock().unwrap_or_else(|p| p.into_inner());
        DharaOperationSnapshot {
            state: self.state(),
            has_total_bytes: u8::from(progress.total_bytes.is_some()),
            total_bytes: progress.total_bytes.unwrap_or(0),
            bytes_transferred: progress.bytes_transferred,
            bytes_per_second: progress.bytes_per_second,
        }
    }

    pub(crate) fn take_string_result(&self) -> Result<Option<String>, FfiFailure> {
        let mut slot = self.result.lock().unwrap_or_else(|p| p.into_inner());
        match slot.take() {
            Some(OperationResult::String(value)) => Ok(Some(value)),
            Some(OperationResult::None) | None => Ok(None),
            Some(OperationResult::Bytes(_)) => Err(FfiFailure::error(
                "operation result contained bytes, not a string",
            )),
        }
    }

    pub(crate) fn take_bytes_result(&self) -> Result<Option<Vec<u8>>, FfiFailure> {
        let mut slot = self.result.lock().unwrap_or_else(|p| p.into_inner());
        match slot.take() {
            Some(OperationResult::Bytes(value)) => Ok(Some(value)),
            Some(OperationResult::None) | None => Ok(None),
            Some(OperationResult::String(_)) => Err(FfiFailure::error(
                "operation result contained a string, not bytes",
            )),
        }
    }

    pub(crate) fn clone_error(&self) -> Option<FfiFailure> {
        self.error
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .as_ref()
            .cloned()
    }
}

impl NativeWriteSession {
    pub(crate) fn write_chunk(&self, bytes: &[u8]) -> Result<(), FfiFailure> {
        if self.completed.load(Ordering::SeqCst) {
            return Err(FfiFailure::error("write session has already completed"));
        }

        let mut slot = self.file.lock().unwrap_or_else(|p| p.into_inner());
        let file = slot
            .as_mut()
            .ok_or_else(|| FfiFailure::error("write session is not open"))?;
        file.write_all(bytes)
            .map_err(|err| FfiFailure::io("write session chunk", &self.path, err))
    }

    pub(crate) fn complete(&self) -> Result<String, FfiFailure> {
        if self.completed.swap(true, Ordering::SeqCst) {
            return Ok(crate::models::path_to_string(&self.path));
        }

        let mut slot = self.file.lock().unwrap_or_else(|p| p.into_inner());
        let mut file = slot
            .take()
            .ok_or_else(|| FfiFailure::error("write session is not open"))?;
        file.flush()
            .map_err(|err| FfiFailure::io("flush write session", &self.path, err))?;
        Ok(crate::models::path_to_string(&self.path))
    }

    pub(crate) fn abort(&self) -> Result<(), FfiFailure> {
        let mut slot = self.file.lock().unwrap_or_else(|p| p.into_inner());
        slot.take();
        if !self.completed.load(Ordering::SeqCst) && self.path.exists() {
            fs::remove_file(&self.path)
                .map_err(|err| FfiFailure::io("abort write session", &self.path, err))?;
        }
        Ok(())
    }
}

pub(crate) fn progress_reporter(
    handle: &Arc<NativeOperationHandle>,
) -> Arc<dyn dhara_storage::ProgressReporter> {
    Arc::new(NativeProgressReporter {
        state: handle.clone(),
    })
}

pub(crate) fn transfer_options(
    handle: &Arc<NativeOperationHandle>,
    overwrite: bool,
) -> TransferOptions {
    TransferOptions {
        overwrite,
        buffer_size: None,
        progress: Some(progress_reporter(handle)),
        cancellation_token: Some(handle.cancellation_token.clone()),
    }
}

pub(crate) fn write_options(
    handle: &Arc<NativeOperationHandle>,
    overwrite: bool,
    create_parent_directories: bool,
) -> WriteOptions {
    WriteOptions {
        overwrite,
        create_parent_directories,
        buffer_size: None,
        progress: Some(progress_reporter(handle)),
        cancellation_token: Some(handle.cancellation_token.clone()),
    }
}

pub(crate) fn delete_options(
    handle: &Arc<NativeOperationHandle>,
    recursive: bool,
) -> DirectoryDeleteOptions {
    DirectoryDeleteOptions {
        recursive,
        cancellation_token: Some(handle.cancellation_token.clone()),
    }
}

pub(crate) fn fail_if_cancelled(handle: &Arc<NativeOperationHandle>) -> Result<(), FfiFailure> {
    if handle.cancellation_token.is_cancelled() {
        return Err(FfiFailure::cancelled("native operation"));
    }

    Ok(())
}

pub(crate) fn spawn_path_operation(
    operation: impl FnOnce(Arc<NativeOperationHandle>) -> Result<OperationResult, FfiFailure>
    + Send
    + 'static,
) -> *mut NativeOperationHandle {
    spawn_operation(operation)
}

pub(crate) fn spawn_operation(
    operation: impl FnOnce(Arc<NativeOperationHandle>) -> Result<OperationResult, FfiFailure>
    + Send
    + 'static,
) -> *mut NativeOperationHandle {
    let handle = NativeOperationHandle::new();
    let worker_state = handle.clone();
    let join = thread::spawn(move || {
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            operation(worker_state.clone())
        })) {
            Ok(Ok(result)) => worker_state.set_completed(result),
            Ok(Err(failure)) => worker_state.set_failure(failure),
            Err(_) => worker_state.set_failure(FfiFailure {
                status: DharaStatus::Panic,
                payload: ErrorPayload {
                    code: "panic",
                    message: "native operation panicked".to_owned(),
                    path: None,
                    operation: None,
                    kind: None,
                    value: None,
                },
            }),
        }
    });

    if let Ok(mut slot) = handle.worker.lock() {
        *slot = Some(join);
    }

    Arc::into_raw(handle) as *mut NativeOperationHandle
}

pub(crate) unsafe fn clone_operation_handle(
    handle: *mut NativeOperationHandle,
) -> Result<Arc<NativeOperationHandle>, FfiFailure> {
    if handle.is_null() {
        return Err(FfiFailure::invalid_argument(
            "handle",
            "operation handle must not be null",
        ));
    }

    let arc = Arc::from_raw(handle);
    let cloned = arc.clone();
    let _ = Arc::into_raw(arc);
    Ok(cloned)
}

pub(crate) unsafe fn with_watch_handle<T>(
    handle: *mut NativeWatchHandle,
    action: impl FnOnce(&NativeWatchHandle) -> Result<T, FfiFailure>,
) -> Result<T, FfiFailure> {
    if handle.is_null() {
        return Err(FfiFailure::invalid_argument(
            "handle",
            "watch handle must not be null",
        ));
    }

    let handle = &*handle;
    action(handle)
}

pub(crate) unsafe fn with_write_session<T>(
    handle: *mut NativeWriteSession,
    action: impl FnOnce(&NativeWriteSession) -> Result<T, FfiFailure>,
) -> Result<T, FfiFailure> {
    if handle.is_null() {
        return Err(FfiFailure::invalid_argument(
            "handle",
            "write session handle must not be null",
        ));
    }

    let handle = &*handle;
    action(handle)
}
