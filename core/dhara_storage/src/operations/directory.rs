//! Sync directory create/copy/move/delete helpers.

use std::fs;
use std::path::{Path, PathBuf};
use std::thread;

use crate::error::StorageError;
use crate::metadata::scan_directory_summary;

use dhara_storage_core::{
    DirectoryDeleteOptions, StorageCancellationToken, StorageProcessEvent, TransferOptions,
};

use super::common::{
    lock_write_targets, normalize_existing_directory, normalize_path,
    prepare_destination_directory, same_volume, validate_single_path_name,
};
use super::transfer::{
    FileIndexCounter, ProcessWriteState, TransferTask, bind_process_cancellation,
    build_transfer_task, process_session_from_options, write_transfer_task,
};
use crate::process::{ProcessOutcome, StorageProcess};

/// Create a single directory level.
pub fn create_directory(path: impl AsRef<Path>) -> Result<PathBuf, StorageError> {
    let path = normalize_path(path)?;
    lock_write_targets(&[&path], || {
        fs::create_dir(&path).map_err(|err| StorageError::io("create directory", &path, err))?;
        Ok(path.clone())
    })
}

/// Create a directory tree.
pub fn create_directory_all(path: impl AsRef<Path>) -> Result<PathBuf, StorageError> {
    let path = normalize_path(path)?;
    lock_write_targets(&[&path], || {
        fs::create_dir_all(&path)
            .map_err(|err| StorageError::io("create directory tree", &path, err))?;
        Ok(path.clone())
    })
}

/// Copy a directory tree to an exact destination path.
///
/// Starts a [`StorageProcess`] immediately and waits for completion.
pub fn copy_directory(
    source: impl AsRef<Path>,
    destination: impl AsRef<Path>,
) -> Result<(), StorageError> {
    start_copy_directory(source, destination).wait_unit()
}

/// Copy a directory tree with overwrite and progress control.
///
/// Starts a [`StorageProcess`] immediately and waits for completion.
pub fn copy_directory_with_options(
    source: impl AsRef<Path>,
    destination: impl AsRef<Path>,
    options: TransferOptions,
) -> Result<(), StorageError> {
    start_copy_directory_with_options(source, destination, options).wait_unit()
}

/// Start a directory copy and return the running [`StorageProcess`] immediately.
pub fn start_copy_directory(
    source: impl AsRef<Path>,
    destination: impl AsRef<Path>,
) -> StorageProcess {
    start_copy_directory_with_options(source, destination, TransferOptions::default())
}

/// Start a directory copy with options and return the running [`StorageProcess`] immediately.
pub fn start_copy_directory_with_options(
    source: impl AsRef<Path>,
    destination: impl AsRef<Path>,
    mut options: TransferOptions,
) -> StorageProcess {
    let source = source.as_ref().to_path_buf();
    let destination = destination.as_ref().to_path_buf();
    let cancellation = bind_process_cancellation(&mut options);
    StorageProcess::spawn(cancellation, move || {
        execute_copy_directory_with_options(source, destination, options).map(|path| {
            ProcessOutcome {
                destination: Some(path),
            }
        })
    })
}

/// Run a directory copy on the calling thread (no process spawn).
///
/// Prefer [`start_copy_directory_with_options`] / [`copy_directory_with_options`]
/// for the process-first public API.
pub fn execute_copy_directory_with_options(
    source: impl AsRef<Path>,
    destination: impl AsRef<Path>,
    options: TransferOptions,
) -> Result<PathBuf, StorageError> {
    let source = normalize_existing_directory(source)?;
    let destination = normalize_path(destination)?;

    if source == destination {
        return Err(StorageError::path_conflict(
            destination,
            "source and destination are the same directory",
        ));
    }

    lock_write_targets(&[&source, &destination], || {
        let session = process_session_from_options(&options);
        session
            .ensure_not_cancelled("copy directory")
            .map_err(StorageError::from)?;
        prepare_destination_directory(&destination, options.overwrite)?;

        if options.overwrite && destination.exists() {
            fs::remove_dir_all(&destination).map_err(|err| {
                StorageError::io("remove directory before overwrite", &destination, err)
            })?;
        }

        fs::create_dir_all(&destination)
            .map_err(|err| StorageError::io("create destination directory", &destination, err))?;

        let summary = if session.has_reporter() {
            Some(scan_directory_summary(&source)?)
        } else {
            None
        };

        if let Some(summary) = summary.as_ref() {
            session.emit(StorageProcessEvent::Started {
                total_bytes: summary.total_size,
                total_files: summary.file_count,
            });
        }

        let producer_count = recommended_producer_count();
        let (task_tx, task_rx) = session.task_queue::<TransferTask>();
        let index = FileIndexCounter::new();
        let cancel = session.cancellation_token().clone();

        let result = thread::scope(|scope| -> Result<(), StorageError> {
            let task_tx_for_walk = task_tx.clone();
            drop(task_tx);
            let source_for_walk = source.clone();
            let destination_for_walk = destination.clone();
            let walk_result = scope.spawn(move || -> Result<(), StorageError> {
                let mut senders = Vec::with_capacity(producer_count);
                let mut slots = Vec::with_capacity(producer_count);
                for _ in 0..producer_count {
                    let (raw_tx, raw_rx) = std::sync::mpsc::sync_channel::<(PathBuf, PathBuf)>(64);
                    senders.push(raw_tx);
                    let task_tx = task_tx_for_walk.clone();
                    let cancel = cancel.clone();
                    let index = index.clone();
                    slots.push(scope.spawn(move || -> Result<(), StorageError> {
                        while let Ok((source_path, dest_path)) = raw_rx.recv() {
                            let file_index = index.next();
                            let task = build_transfer_task(&source_path, &dest_path, file_index)?;
                            task_tx
                                .send(task, &cancel, "copy directory")
                                .map_err(StorageError::from)?;
                        }
                        Ok(())
                    }));
                }
                drop(task_tx_for_walk);

                walk_enqueue_files(&source_for_walk, &destination_for_walk, &senders, &cancel)?;
                drop(senders);

                for slot in slots {
                    slot.join().expect("producer worker panicked")?;
                }
                Ok(())
            });

            let mut state = ProcessWriteState::new(session.clone());
            let consume_result = session.consume_tasks(
                task_rx,
                "copy directory",
                |task| {
                    write_transfer_task(
                        &task,
                        true,
                        options.buffer_size,
                        &mut state,
                        "copy directory",
                    )
                },
                StorageError::from,
            );

            let produce_result: Result<(), StorageError> =
                walk_result.join().expect("directory walk panicked");
            produce_result?;
            consume_result?;
            state.flush_bytes();
            Ok(())
        });

        match result {
            Ok(()) => {
                session.emit(StorageProcessEvent::Completed {
                    destination: Some(destination.clone()),
                });
                Ok(destination.clone())
            }
            Err(err) => {
                if matches!(err, StorageError::Cancelled { .. }) {
                    session.emit(StorageProcessEvent::Cancelled {
                        operation: "copy directory",
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

/// Move a directory tree to an exact destination path.
///
/// Starts a [`StorageProcess`] immediately and waits for completion.
pub fn move_directory(
    source: impl AsRef<Path>,
    destination: impl AsRef<Path>,
) -> Result<(), StorageError> {
    start_move_directory(source, destination).wait_unit()
}

/// Move a directory tree with overwrite and progress control.
///
/// Starts a [`StorageProcess`] immediately and waits for completion.
pub fn move_directory_with_options(
    source: impl AsRef<Path>,
    destination: impl AsRef<Path>,
    options: TransferOptions,
) -> Result<(), StorageError> {
    start_move_directory_with_options(source, destination, options).wait_unit()
}

/// Start a directory move and return the running [`StorageProcess`] immediately.
pub fn start_move_directory(
    source: impl AsRef<Path>,
    destination: impl AsRef<Path>,
) -> StorageProcess {
    start_move_directory_with_options(source, destination, TransferOptions::default())
}

/// Start a directory move with options and return the running [`StorageProcess`] immediately.
pub fn start_move_directory_with_options(
    source: impl AsRef<Path>,
    destination: impl AsRef<Path>,
    mut options: TransferOptions,
) -> StorageProcess {
    let source = source.as_ref().to_path_buf();
    let destination = destination.as_ref().to_path_buf();
    let cancellation = bind_process_cancellation(&mut options);
    StorageProcess::spawn(cancellation, move || {
        execute_move_directory_with_options(source, destination, options).map(|path| {
            ProcessOutcome {
                destination: Some(path),
            }
        })
    })
}

/// Run a directory move on the calling thread (no process spawn).
///
/// Prefer [`start_move_directory_with_options`] / [`move_directory_with_options`]
/// for the process-first public API.
pub fn execute_move_directory_with_options(
    source: impl AsRef<Path>,
    destination: impl AsRef<Path>,
    options: TransferOptions,
) -> Result<PathBuf, StorageError> {
    let source = normalize_existing_directory(source)?;
    let destination = normalize_path(destination)?;

    if source == destination {
        return Ok(source);
    }

    lock_write_targets(&[&source, &destination], || {
        super::common::ensure_not_cancelled(options.cancellation_token.as_ref(), "move directory")?;
        prepare_destination_directory(&destination, options.overwrite)?;

        if same_volume(&source, &destination) {
            if options.overwrite && destination.exists() {
                fs::remove_dir_all(&destination).map_err(|err| {
                    StorageError::io("remove directory before overwrite", &destination, err)
                })?;
            }

            fs::rename(&source, &destination)
                .map_err(|err| StorageError::io("move directory to", &destination, err))?;

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

        execute_copy_directory_with_options(&source, &destination, options.clone())?;
        fs::remove_dir_all(&source)
            .map_err(|err| StorageError::io("delete source directory after move", &source, err))?;
        Ok(destination.clone())
    })
}

/// Rename a directory in place inside its current parent directory.
pub fn rename_directory(source: impl AsRef<Path>, new_name: &str) -> Result<PathBuf, StorageError> {
    validate_single_path_name(new_name, "directory")?;

    let source = normalize_existing_directory(source)?;
    let destination = source
        .parent()
        .map(|parent| parent.join(new_name))
        .ok_or_else(|| {
            StorageError::path_conflict(source.clone(), "directory has no parent directory")
        })?;

    execute_move_directory_with_options(source, destination, TransferOptions::default())
}

/// Delete a directory tree recursively.
pub fn delete_directory(path: impl AsRef<Path>) -> Result<(), StorageError> {
    delete_directory_with_options(path, DirectoryDeleteOptions::default())
}

/// Delete a directory either recursively or only when empty.
pub fn delete_directory_with_options(
    path: impl AsRef<Path>,
    options: DirectoryDeleteOptions,
) -> Result<(), StorageError> {
    let path = normalize_existing_directory(path)?;

    lock_write_targets(&[&path], || {
        if options.recursive {
            if options.cancellation_token.is_some() {
                delete_directory_recursive(&path, options.cancellation_token.as_ref())
            } else {
                fs::remove_dir_all(&path)
                    .map_err(|err| StorageError::io("delete directory tree", &path, err))
            }
        } else {
            fs::remove_dir(&path).map_err(|err| StorageError::io("delete directory", &path, err))
        }
    })
}

fn recommended_producer_count() -> usize {
    thread::available_parallelism()
        .map(|n| n.get().clamp(2, 8))
        .unwrap_or(4)
}

fn walk_enqueue_files(
    source: &Path,
    destination: &Path,
    senders: &[std::sync::mpsc::SyncSender<(PathBuf, PathBuf)>],
    cancellation: &StorageCancellationToken,
) -> Result<(), StorageError> {
    let mut next_sender = 0usize;
    walk_enqueue_recursive(source, destination, senders, &mut next_sender, cancellation)
}

fn walk_enqueue_recursive(
    source: &Path,
    destination: &Path,
    senders: &[std::sync::mpsc::SyncSender<(PathBuf, PathBuf)>],
    next_sender: &mut usize,
    cancellation: &StorageCancellationToken,
) -> Result<(), StorageError> {
    super::common::ensure_not_cancelled(Some(cancellation), "copy directory")?;

    for entry in fs::read_dir(source)
        .map_err(|err| StorageError::io("read directory for copy", source, err))?
    {
        super::common::ensure_not_cancelled(Some(cancellation), "copy directory")?;
        let entry = entry
            .map_err(|err| StorageError::io("enumerate directory entries for copy", source, err))?;
        let file_type = entry
            .file_type()
            .map_err(|err| StorageError::io("read file type for copy", entry.path(), err))?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());

        if file_type.is_dir() {
            fs::create_dir_all(&destination_path).map_err(|err| {
                StorageError::io(
                    "create destination directory during copy",
                    &destination_path,
                    err,
                )
            })?;
            walk_enqueue_recursive(
                &source_path,
                &destination_path,
                senders,
                next_sender,
                cancellation,
            )?;
            continue;
        }

        if file_type.is_file() {
            let sender = &senders[*next_sender % senders.len()];
            *next_sender += 1;
            sender
                .send((source_path, destination_path))
                .map_err(|_| StorageError::from(dhara_storage_core::ProcessError::Disconnected))?;
        }
    }

    Ok(())
}

fn delete_directory_recursive(
    path: &Path,
    cancellation_token: Option<&StorageCancellationToken>,
) -> Result<(), StorageError> {
    super::common::ensure_not_cancelled(cancellation_token, "delete directory")?;
    for entry in fs::read_dir(path)
        .map_err(|err| StorageError::io("read directory for delete", path, err))?
    {
        super::common::ensure_not_cancelled(cancellation_token, "delete directory")?;
        let entry = entry
            .map_err(|err| StorageError::io("enumerate directory entries for delete", path, err))?;
        let entry_path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|err| StorageError::io("read file type for delete", &entry_path, err))?;

        if file_type.is_dir() {
            delete_directory_recursive(&entry_path, cancellation_token)?;
        } else {
            fs::remove_file(&entry_path).map_err(|err| {
                StorageError::io("delete file during directory delete", &entry_path, err)
            })?;
        }
    }

    fs::remove_dir(path).map_err(|err| StorageError::io("delete directory tree", path, err))
}
