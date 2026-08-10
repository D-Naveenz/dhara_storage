//! Transfer tasks and ProcessSession-backed copy helpers.

use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use dhara_storage_core::{
    BytesEventThrottle, ProcessSession, SharedProcessEventReporter, StorageCancellationToken,
    StorageProcessEvent, TransferOptions,
};

use crate::error::StorageError;

use super::common::{
    choose_buffer_size, open_destination_file, open_source_file, prepare_destination_file,
};

/// One file transfer unit queued for the single writer consumer.
#[derive(Debug, Clone)]
pub(crate) struct TransferTask {
    pub source_path: PathBuf,
    pub dest_path: PathBuf,
    pub file_size: u64,
    pub file_index: u64,
}

/// Mutable byte-progress state shared by a process writer.
pub(crate) struct ProcessWriteState {
    pub session: ProcessSession,
    pub throttle: BytesEventThrottle,
    pub bytes_transferred: u64,
}

impl ProcessWriteState {
    pub fn new(session: ProcessSession) -> Self {
        Self {
            session,
            throttle: BytesEventThrottle::default(),
            bytes_transferred: 0,
        }
    }

    pub fn emit_bytes_if_due(&mut self) {
        let now = Instant::now();
        if self.throttle.should_emit(self.bytes_transferred, now) {
            self.session.emit(StorageProcessEvent::Bytes {
                bytes_transferred: self.bytes_transferred,
            });
            self.throttle.mark_emitted(self.bytes_transferred, now);
        }
    }

    pub fn flush_bytes(&mut self) {
        self.session.emit(StorageProcessEvent::Bytes {
            bytes_transferred: self.bytes_transferred,
        });
        self.throttle
            .mark_emitted(self.bytes_transferred, Instant::now());
    }

    pub fn begin_item(&mut self, task: &TransferTask) {
        self.flush_bytes_if_any();
        self.throttle.force_next();
        self.session.emit(StorageProcessEvent::CurrentItem {
            path: task.dest_path.clone(),
            file_size: task.file_size,
            file_index: task.file_index,
        });
    }

    fn flush_bytes_if_any(&mut self) {
        if self.bytes_transferred > 0 {
            self.flush_bytes();
        }
    }
}

pub(crate) fn process_session_from_options(options: &TransferOptions) -> ProcessSession {
    ProcessSession::new(
        options.cancellation_token.clone(),
        options.progress.clone(),
    )
}

pub(crate) fn build_transfer_task(
    source: &Path,
    destination: &Path,
    file_index: u64,
) -> Result<TransferTask, StorageError> {
    let file_size = fs::metadata(source)
        .map_err(|err| StorageError::io("read metadata for", source, err))?
        .len();

    Ok(TransferTask {
        source_path: source.to_path_buf(),
        dest_path: destination.to_path_buf(),
        file_size,
        file_index,
    })
}

pub(crate) fn write_transfer_task(
    task: &TransferTask,
    overwrite: bool,
    buffer_size: Option<usize>,
    state: &mut ProcessWriteState,
    operation: &'static str,
) -> Result<(), StorageError> {
    state
        .session
        .ensure_not_cancelled(operation)
        .map_err(StorageError::from)?;
    state.begin_item(task);

    prepare_destination_file(&task.dest_path, overwrite, true)?;
    let chosen = choose_buffer_size(Some(task.file_size), buffer_size);
    let mut source_file = open_source_file(&task.source_path)?;
    let mut destination_file = open_destination_file(&task.dest_path, overwrite)?;

    // Pre-allocate destination length to reduce fragmentation on large files.
    destination_file
        .set_len(task.file_size)
        .map_err(|err| StorageError::io("pre-allocate destination file", &task.dest_path, err))?;

    copy_reader_to_process_writer(
        &mut source_file,
        &mut destination_file,
        chosen,
        state,
        operation,
    )?;
    state.flush_bytes();
    Ok(())
}

pub(crate) fn copy_reader_to_process_writer<R, W>(
    reader: &mut R,
    writer: &mut W,
    buffer_size: usize,
    state: &mut ProcessWriteState,
    operation: &'static str,
) -> Result<u64, StorageError>
where
    R: Read,
    W: Write,
{
    let mut buffer = vec![0u8; buffer_size];
    let mut file_transferred = 0u64;

    loop {
        state
            .session
            .ensure_not_cancelled(operation)
            .map_err(StorageError::from)?;
        let read = reader
            .read(&mut buffer)
            .map_err(|err| StorageError::reader_io("read from", err))?;
        if read == 0 {
            break;
        }

        writer
            .write_all(&buffer[..read])
            .map_err(|err| StorageError::reader_io("write to", err))?;
        file_transferred += read as u64;
        state.bytes_transferred += read as u64;
        state.emit_bytes_if_due();
    }

    Ok(file_transferred)
}

/// Adapter for read/write option bundles that still use a process event reporter.
pub(crate) fn copy_reader_to_writer_events<R, W>(
    reader: &mut R,
    writer: &mut W,
    total_bytes: Option<u64>,
    buffer_size: usize,
    progress: Option<&SharedProcessEventReporter>,
    cancellation_token: Option<&StorageCancellationToken>,
    operation: &'static str,
) -> Result<u64, StorageError>
where
    R: Read,
    W: Write,
{
    if progress.is_none() && cancellation_token.is_none() {
        return std::io::copy(reader, writer)
            .map_err(|err| StorageError::reader_io("copy from", err));
    }

    let process = ProcessSession::new(cancellation_token.cloned(), progress.cloned());
    if let Some(total) = total_bytes {
        process.emit(StorageProcessEvent::Started {
            total_bytes: total,
            total_files: 1,
        });
    } else {
        process.emit(StorageProcessEvent::Started {
            total_bytes: 0,
            total_files: 1,
        });
    }

    let mut state = ProcessWriteState::new(process);
    let transferred =
        copy_reader_to_process_writer(reader, writer, buffer_size, &mut state, operation)?;
    state.flush_bytes();
    Ok(transferred)
}

/// Ensure transfer options carry a cancellation token shared with a spawned process.
pub(crate) fn bind_process_cancellation(
    options: &mut TransferOptions,
) -> StorageCancellationToken {
    if let Some(token) = options.cancellation_token.clone() {
        token
    } else {
        let token = StorageCancellationToken::default();
        options.cancellation_token = Some(token.clone());
        token
    }
}

/// Shared atomic file index allocator for producer pools.
#[derive(Clone)]
pub(crate) struct FileIndexCounter(Arc<AtomicU64>);

impl FileIndexCounter {
    pub fn new() -> Self {
        Self(Arc::new(AtomicU64::new(0)))
    }

    pub fn next(&self) -> u64 {
        self.0.fetch_add(1, Ordering::SeqCst)
    }
}
