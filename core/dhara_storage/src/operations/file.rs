//! Sync file read/write/copy/move/delete helpers.

use std::fs;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};

use tracing::{debug, info};

use crate::error::StorageError;

use dhara_storage_core::{ReadOptions, StorageProcessEvent, TransferOptions, WriteOptions};

use super::common::{
    choose_buffer_size, copy_reader_to_writer, lock_write_targets, normalize_existing_file,
    normalize_path, open_destination_file, open_source_file, prepare_destination_file, same_volume,
    validate_single_path_name,
};
use super::transfer::{
    ProcessWriteState, build_transfer_task, storage_process_from_options, write_transfer_task,
};

/// Copy a file to an exact destination path.
pub fn copy_file(
    source: impl AsRef<Path>,
    destination: impl AsRef<Path>,
) -> Result<PathBuf, StorageError> {
    copy_file_with_options(source, destination, TransferOptions::default())
}

/// Copy a file to an exact destination path with overwrite and progress control.
pub fn copy_file_with_options(
    source: impl AsRef<Path>,
    destination: impl AsRef<Path>,
    options: TransferOptions,
) -> Result<PathBuf, StorageError> {
    let source = normalize_existing_file(source)?;
    let destination = normalize_path(destination)?;
    info!(
        target: "dhara_storage::operations::file",
        source = %source.display(),
        destination = %destination.display(),
        overwrite = options.overwrite,
        progress = options.progress.is_some(),
        cancellable = options.cancellation_token.is_some(),
        "copying file"
    );
    if source == destination {
        return Err(StorageError::path_conflict(
            destination,
            "source and destination are the same file",
        ));
    }

    lock_write_targets(&[&source, &destination], || {
        let process = storage_process_from_options(&options);
        process
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
            info!(
                target: "dhara_storage::operations::file",
                destination = %destination.display(),
                "copied file using direct fs::copy fast path"
            );
            return Ok(destination.clone());
        }

        let task = build_transfer_task(&source, &destination, 0)?;
        process.emit(StorageProcessEvent::Started {
            total_bytes: task.file_size,
            total_files: 1,
        });

        let mut state = ProcessWriteState::new(process.clone());
        match write_transfer_task(
            &task,
            options.overwrite,
            options.buffer_size,
            &mut state,
            "copy file",
        ) {
            Ok(()) => {
                process.emit(StorageProcessEvent::Completed {
                    destination: Some(destination.clone()),
                });
                info!(
                    target: "dhara_storage::operations::file",
                    destination = %destination.display(),
                    total_bytes = task.file_size,
                    "copied file with storage process"
                );
                Ok(destination.clone())
            }
            Err(err) => {
                if matches!(err, StorageError::Cancelled { .. }) {
                    process.emit(StorageProcessEvent::Cancelled {
                        operation: "copy file",
                    });
                } else {
                    process.emit(StorageProcessEvent::Failed {
                        message: err.to_string(),
                    });
                }
                Err(err)
            }
        }
    })
}

/// Move a file to an exact destination path.
pub fn move_file(
    source: impl AsRef<Path>,
    destination: impl AsRef<Path>,
) -> Result<PathBuf, StorageError> {
    move_file_with_options(source, destination, TransferOptions::default())
}

/// Move a file to an exact destination path with overwrite and progress control.
pub fn move_file_with_options(
    source: impl AsRef<Path>,
    destination: impl AsRef<Path>,
    options: TransferOptions,
) -> Result<PathBuf, StorageError> {
    let source = normalize_existing_file(source)?;
    let destination = normalize_path(destination)?;
    info!(
        target: "dhara_storage::operations::file",
        source = %source.display(),
        destination = %destination.display(),
        overwrite = options.overwrite,
        progress = options.progress.is_some(),
        cancellable = options.cancellation_token.is_some(),
        "moving file"
    );

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

            info!(
                target: "dhara_storage::operations::file",
                destination = %destination.display(),
                "moved file using same-volume rename"
            );
            return Ok(destination.clone());
        }

        copy_file_with_options(&source, &destination, options.clone())?;
        fs::remove_file(&source)
            .map_err(|err| StorageError::io("delete source file after move", &source, err))?;
        info!(
            target: "dhara_storage::operations::file",
            source = %source.display(),
            destination = %destination.display(),
            "moved file using copy/delete fallback"
        );
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

    move_file(&source, &destination)
}

/// Delete a file from disk.
pub fn delete_file(path: impl AsRef<Path>) -> Result<(), StorageError> {
    let path = normalize_existing_file(path)?;
    info!(
        target: "dhara_storage::operations::file",
        path = %path.display(),
        "deleting file"
    );

    lock_write_targets(&[&path], || {
        fs::remove_file(&path).map_err(|err| StorageError::io("delete file", &path, err))
    })
}

/// Read the entire file into memory.
pub fn read_file(path: impl AsRef<Path>) -> Result<Vec<u8>, StorageError> {
    debug!(
        target: "dhara_storage::operations::file",
        path = %path.as_ref().display(),
        "reading file bytes"
    );
    read_file_with_options(path, ReadOptions::default())
}

/// Read the entire file as UTF-8 text.
pub fn read_file_to_string(path: impl AsRef<Path>) -> Result<String, StorageError> {
    let path = normalize_existing_file(path)?;
    debug!(
        target: "dhara_storage::operations::file",
        path = %path.display(),
        "reading file text"
    );
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
    info!(
        target: "dhara_storage::operations::file",
        destination = %destination.display(),
        overwrite = options.overwrite,
        create_parent_directories = options.create_parent_directories,
        progress = options.progress.is_some(),
        cancellable = options.cancellation_token.is_some(),
        "writing file from reader"
    );

    lock_write_targets(&[&destination], || {
        prepare_destination_file(
            &destination,
            options.overwrite,
            options.create_parent_directories,
        )?;

        if options.progress.is_none() && options.cancellation_token.is_none() {
            let mut file = open_destination_file(&destination, options.overwrite)?;
            std::io::copy(reader, &mut file)
                .map_err(|err| StorageError::reader_io("write file from reader", err))?;
            info!(
                target: "dhara_storage::operations::file",
                destination = %destination.display(),
                "wrote file using direct std::io::copy fast path"
            );
            return Ok(destination.clone());
        }

        let buffer_size = choose_buffer_size(None, options.buffer_size);
        let mut destination_file = open_destination_file(&destination, options.overwrite)?;
        copy_reader_to_writer(
            reader,
            &mut destination_file,
            None,
            buffer_size,
            options.progress.as_ref(),
            options.cancellation_token.as_ref(),
            "write file",
        )?;

        info!(
            target: "dhara_storage::operations::file",
            destination = %destination.display(),
            buffer_size,
            "wrote file using buffered transfer"
        );
        Ok(destination.clone())
    })
}

pub(crate) fn read_file_with_options(
    path: impl AsRef<Path>,
    options: ReadOptions,
) -> Result<Vec<u8>, StorageError> {
    let path = normalize_existing_file(path)?;
    debug!(
        target: "dhara_storage::operations::file",
        path = %path.display(),
        progress = options.progress.is_some(),
        cancellable = options.cancellation_token.is_some(),
        "reading file with options"
    );

    if options.progress.is_none() && options.cancellation_token.is_none() {
        return fs::read(&path).map_err(|err| StorageError::io("read file", &path, err));
    }

    let total_bytes = fs::metadata(&path)
        .map_err(|err| StorageError::io("read metadata for", &path, err))?
        .len();
    let buffer_size = choose_buffer_size(Some(total_bytes), options.buffer_size);
    let mut source = open_source_file(&path)?;
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
    debug!(
        target: "dhara_storage::operations::file",
        path = %path.display(),
        total_bytes,
        buffer_size,
        "completed buffered file read"
    );
    Ok(buffer)
}
