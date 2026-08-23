//! Sync file read/write/copy/move/delete helpers.

use std::fs;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};

use crate::error::StorageError;

use super::common::{
    choose_buffer_size, copy_reader_to_writer, lock_write_targets, normalize_existing_file,
    normalize_path, prepare_destination_file, same_volume, validate_single_path_name,
};
use super::open::{open_for_read, open_for_write};
use super::transfer::{
    ProcessWriteState, bind_process_cancellation, build_transfer_task,
    process_session_from_options, write_transfer_task,
};
use crate::process::{ProcessOutcome, StorageProcess};
use dhara_storage_core::{
    OpenReadOptions, OpenWriteOptions, ReadOptions, StorageProcessEvent, TransferOptions,
    WriteOptions,
};

/// Copy a file to an exact destination path.
///
/// Starts a [`StorageProcess`] immediately and waits for completion.
pub fn copy_file(
    source: impl AsRef<Path>,
    destination: impl AsRef<Path>,
) -> Result<(), StorageError> {
    start_copy_file(source, destination).wait_unit()
}

/// Copy a file with overwrite and progress control.
///
/// Starts a [`StorageProcess`] immediately and waits for completion.
pub fn copy_file_with_options(
    source: impl AsRef<Path>,
    destination: impl AsRef<Path>,
    options: TransferOptions,
) -> Result<(), StorageError> {
    start_copy_file_with_options(source, destination, options).wait_unit()
}

/// Start a file copy and return the running [`StorageProcess`] immediately.
pub fn start_copy_file(source: impl AsRef<Path>, destination: impl AsRef<Path>) -> StorageProcess {
    start_copy_file_with_options(source, destination, TransferOptions::default())
}

/// Start a file copy with options and return the running [`StorageProcess`] immediately.
pub fn start_copy_file_with_options(
    source: impl AsRef<Path>,
    destination: impl AsRef<Path>,
    mut options: TransferOptions,
) -> StorageProcess {
    let source = source.as_ref().to_path_buf();
    let destination = destination.as_ref().to_path_buf();
    let user_cancel = options.cancellation_token.is_some();
    let user_progress = options.progress.is_some();
    let cancellation = bind_process_cancellation(&mut options);
    StorageProcess::spawn(cancellation, move || {
        // Auto-bound cancel is for the handle only; keep the direct fs::copy fast path
        // unless the caller asked for progress or cooperative cancellation.
        if !user_progress && !user_cancel {
            options.cancellation_token = None;
        }
        execute_copy_file_with_options(source, destination, options).map(|path| ProcessOutcome {
            destination: Some(path),
        })
    })
}

/// Run a file copy on the calling thread (no process spawn).
///
/// Prefer [`start_copy_file_with_options`] / [`copy_file_with_options`] for the
/// process-first public API. Interop hosts that already own a worker thread may
/// call this engine entry directly.
pub fn execute_copy_file_with_options(
    source: impl AsRef<Path>,
    destination: impl AsRef<Path>,
    options: TransferOptions,
) -> Result<PathBuf, StorageError> {
    let source = normalize_existing_file(source)?;
    let destination = normalize_path(destination)?;
    if source == destination {
        return Err(StorageError::path_conflict(
            destination,
            "source and destination are the same file",
        ));
    }

    lock_write_targets(&[&source, &destination], || {
        let session = process_session_from_options(&options);
        session
            .ensure_not_cancelled("copy file")
            .map_err(StorageError::from)?;

        if options.progress.is_none() && options.cancellation_token.is_none() {
            prepare_destination_file(&destination, options.overwrite, true)?;
            if options.overwrite && destination.exists() {
                fs::remove_file(&destination).map_err(|err| {
                    StorageError::io("remove file before overwrite", &destination, err)
                })?;
            }

            fs::copy(&source, &destination)
                .map_err(|err| StorageError::io("copy file to", &destination, err))?;
            return Ok(destination.clone());
        }

        let task = build_transfer_task(&source, &destination, 0)?;
        session.emit(StorageProcessEvent::Started {
            total_bytes: task.file_size,
            total_files: 1,
        });

        let mut state = ProcessWriteState::new(session.clone());
        match write_transfer_task(
            &task,
            options.overwrite,
            options.buffer_size,
            &mut state,
            "copy file",
        ) {
            Ok(()) => {
                session.emit(StorageProcessEvent::Completed {
                    destination: Some(destination.clone()),
                });
                Ok(destination.clone())
            }
            Err(err) => {
                if matches!(err, StorageError::Cancelled { .. }) {
                    session.emit(StorageProcessEvent::Cancelled {
                        operation: "copy file",
                    });
                } else {
                    session.emit(StorageProcessEvent::Failed {
                        message: err.to_string(),
                    });
                }
                Err(err)
            }
        }
    })
}

/// Move a file to an exact destination path.
///
/// Starts a [`StorageProcess`] immediately and waits for completion.
pub fn move_file(
    source: impl AsRef<Path>,
    destination: impl AsRef<Path>,
) -> Result<(), StorageError> {
    start_move_file(source, destination).wait_unit()
}

/// Move a file with overwrite and progress control.
///
/// Starts a [`StorageProcess`] immediately and waits for completion.
pub fn move_file_with_options(
    source: impl AsRef<Path>,
    destination: impl AsRef<Path>,
    options: TransferOptions,
) -> Result<(), StorageError> {
    start_move_file_with_options(source, destination, options).wait_unit()
}

/// Start a file move and return the running [`StorageProcess`] immediately.
pub fn start_move_file(source: impl AsRef<Path>, destination: impl AsRef<Path>) -> StorageProcess {
    start_move_file_with_options(source, destination, TransferOptions::default())
}

/// Start a file move with options and return the running [`StorageProcess`] immediately.
pub fn start_move_file_with_options(
    source: impl AsRef<Path>,
    destination: impl AsRef<Path>,
    mut options: TransferOptions,
) -> StorageProcess {
    let source = source.as_ref().to_path_buf();
    let destination = destination.as_ref().to_path_buf();
    let user_cancel = options.cancellation_token.is_some();
    let user_progress = options.progress.is_some();
    let cancellation = bind_process_cancellation(&mut options);
    StorageProcess::spawn(cancellation, move || {
        if !user_progress && !user_cancel {
            options.cancellation_token = None;
        }
        execute_move_file_with_options(source, destination, options).map(|path| ProcessOutcome {
            destination: Some(path),
        })
    })
}

/// Run a file move on the calling thread (no process spawn).
///
/// Prefer [`start_move_file_with_options`] / [`move_file_with_options`] for the
/// process-first public API.
pub fn execute_move_file_with_options(
    source: impl AsRef<Path>,
    destination: impl AsRef<Path>,
    options: TransferOptions,
) -> Result<PathBuf, StorageError> {
    let source = normalize_existing_file(source)?;
    let destination = normalize_path(destination)?;

    if source == destination {
        return Ok(source);
    }

    lock_write_targets(&[&source, &destination], || {
        super::common::ensure_not_cancelled(options.cancellation_token.as_ref(), "move file")?;
        prepare_destination_file(&destination, options.overwrite, true)?;

        if same_volume(&source, &destination) {
            if options.overwrite && destination.exists() {
                fs::remove_file(&destination).map_err(|err| {
                    StorageError::io("remove file before overwrite", &destination, err)
                })?;
            }

            fs::rename(&source, &destination)
                .map_err(|err| StorageError::io("move file to", &destination, err))?;

            if let Some(progress) = options.progress.as_ref() {
                progress.report(StorageProcessEvent::Started {
                    total_bytes: 1,
                    total_files: 1,
                });
                progress.report(StorageProcessEvent::Completed {
                    destination: Some(destination.clone()),
                });
            }

            return Ok(destination.clone());
        }

        execute_copy_file_with_options(&source, &destination, options.clone())?;
        fs::remove_file(&source)
            .map_err(|err| StorageError::io("delete source file after move", &source, err))?;
        Ok(destination.clone())
    })
}

/// Rename a file in place inside its current parent directory.
pub fn rename_file(source: impl AsRef<Path>, new_name: &str) -> Result<PathBuf, StorageError> {
    validate_single_path_name(new_name, "file")?;

    let source = normalize_existing_file(source)?;
    let destination = source
        .parent()
        .map(|parent| parent.join(new_name))
        .ok_or_else(|| {
            StorageError::path_conflict(source.clone(), "file has no parent directory")
        })?;

    execute_move_file_with_options(source, destination, TransferOptions::default())
}

/// Delete a file from disk.
pub fn delete_file(path: impl AsRef<Path>) -> Result<(), StorageError> {
    let path = normalize_existing_file(path)?;

    lock_write_targets(&[&path], || {
        fs::remove_file(&path).map_err(|err| StorageError::io("delete file", &path, err))
    })
}

/// Read the entire file into memory.
pub fn read_file(path: impl AsRef<Path>) -> Result<Vec<u8>, StorageError> {
    read_file_with_options(path, ReadOptions::default())
}

/// Read the entire file as UTF-8 text.
pub fn read_file_to_string(path: impl AsRef<Path>) -> Result<String, StorageError> {
    let path = normalize_existing_file(path)?;
    fs::read_to_string(&path).map_err(|err| StorageError::io("read file as text", &path, err))
}

/// Write bytes to a file, creating parent directories by default.
pub fn write_file(
    path: impl AsRef<Path>,
    bytes: impl AsRef<[u8]>,
) -> Result<PathBuf, StorageError> {
    let mut cursor = Cursor::new(bytes.as_ref().to_vec());
    write_file_from_reader(path, &mut cursor, WriteOptions::default())
}

/// Write UTF-8 text to a file, creating parent directories by default.
pub fn write_file_string(
    path: impl AsRef<Path>,
    text: impl AsRef<str>,
) -> Result<PathBuf, StorageError> {
    write_file(path, text.as_ref().as_bytes())
}

/// Stream bytes into a file with overwrite and progress control.
pub fn write_file_from_reader(
    path: impl AsRef<Path>,
    reader: &mut impl Read,
    options: WriteOptions,
) -> Result<PathBuf, StorageError> {
    let destination = normalize_path(path)?;

    lock_write_targets(&[&destination], || {
        prepare_destination_file(
            &destination,
            options.overwrite,
            options.create_parent_directories,
        )?;

        if options.progress.is_none() && options.cancellation_token.is_none() {
            let mut file = open_for_write(
                &destination,
                OpenWriteOptions {
                    overwrite: options.overwrite,
                    create_parent_directories: false,
                    ..OpenWriteOptions::default()
                },
            )?;
            std::io::copy(reader, &mut file)
                .map_err(|err| StorageError::reader_io("write file from reader", err))?;
            return Ok(destination.clone());
        }

        let buffer_size = choose_buffer_size(None, options.buffer_size);
        let mut destination_file = open_for_write(
            &destination,
            OpenWriteOptions {
                overwrite: options.overwrite,
                create_parent_directories: false,
                ..OpenWriteOptions::default()
            },
        )?;
        copy_reader_to_writer(
            reader,
            &mut destination_file,
            None,
            buffer_size,
            options.progress.as_ref(),
            options.cancellation_token.as_ref(),
            "write file",
        )?;

        Ok(destination.clone())
    })
}

pub(crate) fn read_file_with_options(
    path: impl AsRef<Path>,
    options: ReadOptions,
) -> Result<Vec<u8>, StorageError> {
    let path = normalize_existing_file(path)?;

    if options.progress.is_none() && options.cancellation_token.is_none() {
        return fs::read(&path).map_err(|err| StorageError::io("read file", &path, err));
    }

    let total_bytes = fs::metadata(&path)
        .map_err(|err| StorageError::io("read metadata for", &path, err))?
        .len();
    let buffer_size = choose_buffer_size(Some(total_bytes), options.buffer_size);
    let mut source = open_for_read(&path, OpenReadOptions::default())?;
    let mut buffer = Vec::with_capacity(total_bytes as usize);
    copy_reader_to_writer(
        &mut source,
        &mut buffer,
        Some(total_bytes),
        buffer_size,
        options.progress.as_ref(),
        options.cancellation_token.as_ref(),
        "read file",
    )?;
    Ok(buffer)
}
