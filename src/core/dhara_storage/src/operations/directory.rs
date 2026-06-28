use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;

use tracing::{debug, info};

use crate::error::StorageError;
use crate::info::DirectoryInfo;

use super::common::{
    DirectoryDeleteOptions, SharedProgressReporter, StorageProgress, TransferOptions,
    choose_buffer_size, lock_write_targets, normalize_existing_directory, normalize_path,
    open_destination_file, open_source_file, prepare_destination_directory,
    prepare_destination_file, report_progress, same_volume, validate_single_path_name,
};
use super::file::copy_file_with_options;

/// Create a single directory level.
pub fn create_directory(path: impl AsRef<Path>) -> Result<PathBuf, StorageError> {
    let path = normalize_path(path)?;
    info!(target: "dhara_storage::operations::directory", path = %path.display(), "creating directory");
    lock_write_targets(&[&path], || {
        fs::create_dir(&path).map_err(|err| StorageError::io("create directory", &path, err))?;
        Ok(path.clone())
    })
}

/// Create a directory tree.
pub fn create_directory_all(path: impl AsRef<Path>) -> Result<PathBuf, StorageError> {
    let path = normalize_path(path)?;
    info!(target: "dhara_storage::operations::directory", path = %path.display(), "creating directory tree");
    lock_write_targets(&[&path], || {
        fs::create_dir_all(&path)
            .map_err(|err| StorageError::io("create directory tree", &path, err))?;
        Ok(path.clone())
    })
}

/// Copy a directory tree to an exact destination path.
pub fn copy_directory(
    source: impl AsRef<Path>,
    destination: impl AsRef<Path>,
) -> Result<PathBuf, StorageError> {
    copy_directory_with_options(source, destination, TransferOptions::default())
}

/// Copy a directory tree to an exact destination path with overwrite and progress control.
pub fn copy_directory_with_options(
    source: impl AsRef<Path>,
    destination: impl AsRef<Path>,
    options: TransferOptions,
) -> Result<PathBuf, StorageError> {
    let source = normalize_existing_directory(source)?;
    let destination = normalize_path(destination)?;
    info!(
        target: "dhara_storage::operations::directory",
        source = %source.display(),
        destination = %destination.display(),
        overwrite = options.overwrite,
        progress = options.progress.is_some(),
        cancellable = options.cancellation_token.is_some(),
        "copying directory"
    );

    if source == destination {
        return Err(StorageError::path_conflict(
            destination,
            "source and destination are the same directory",
        ));
    }

    lock_write_targets(&[&source, &destination], || {
        super::common::ensure_not_cancelled(options.cancellation_token.as_ref(), "copy directory")?;
        prepare_destination_directory(&destination, options.overwrite)?;

        if options.overwrite && destination.exists() {
            fs::remove_dir_all(&destination).map_err(|err| {
                StorageError::io("remove directory before overwrite", &destination, err)
            })?;
        }

        fs::create_dir_all(&destination)
            .map_err(|err| StorageError::io("create destination directory", &destination, err))?;

        let total_bytes = if options.progress.is_some() {
            Some(DirectoryInfo::from_path(&source)?.summary()?.total_size)
        } else {
            None
        };
        let mut progress = DirectoryProgress::new(
            total_bytes,
            options.progress.clone(),
            options.cancellation_token.clone(),
        );
        copy_directory_recursive(&source, &destination, &options, &mut progress)?;
        info!(
            target: "dhara_storage::operations::directory",
            source = %source.display(),
            destination = %destination.display(),
            total_bytes = total_bytes.unwrap_or_default(),
            "completed directory copy"
        );
        Ok(destination.clone())
    })
}

/// Move a directory tree to an exact destination path.
pub fn move_directory(
    source: impl AsRef<Path>,
    destination: impl AsRef<Path>,
) -> Result<PathBuf, StorageError> {
    move_directory_with_options(source, destination, TransferOptions::default())
}

/// Move a directory tree to an exact destination path with overwrite and progress control.
pub fn move_directory_with_options(
    source: impl AsRef<Path>,
    destination: impl AsRef<Path>,
    options: TransferOptions,
) -> Result<PathBuf, StorageError> {
    let source = normalize_existing_directory(source)?;
    let destination = normalize_path(destination)?;
    info!(
        target: "dhara_storage::operations::directory",
        source = %source.display(),
        destination = %destination.display(),
        overwrite = options.overwrite,
        progress = options.progress.is_some(),
        cancellable = options.cancellation_token.is_some(),
        "moving directory"
    );

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
                progress.report(StorageProgress {
                    total_bytes: Some(1),
                    bytes_transferred: 1,
                    bytes_per_second: 0.0,
                });
            }

            info!(
                target: "dhara_storage::operations::directory",
                destination = %destination.display(),
                "moved directory using same-volume rename"
            );
            return Ok(destination.clone());
        }

        copy_directory_with_options(&source, &destination, options.clone())?;
        fs::remove_dir_all(&source)
            .map_err(|err| StorageError::io("delete source directory after move", &source, err))?;
        info!(
            target: "dhara_storage::operations::directory",
            source = %source.display(),
            destination = %destination.display(),
            "moved directory using copy/delete fallback"
        );
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

    move_directory(&source, &destination)
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
    info!(
        target: "dhara_storage::operations::directory",
        path = %path.display(),
        recursive = options.recursive,
        cancellable = options.cancellation_token.is_some(),
        "deleting directory"
    );

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

fn copy_directory_recursive(
    source: &Path,
    destination: &Path,
    options: &TransferOptions,
    progress: &mut DirectoryProgress,
) -> Result<(), StorageError> {
    debug!(
        target: "dhara_storage::operations::directory",
        source = %source.display(),
        destination = %destination.display(),
        "walking directory for copy"
    );
    for entry in fs::read_dir(source)
        .map_err(|err| StorageError::io("read directory for copy", source, err))?
    {
        super::common::ensure_not_cancelled(options.cancellation_token.as_ref(), "copy directory")?;
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
            copy_directory_recursive(&source_path, &destination_path, options, progress)?;
            continue;
        }

        if file_type.is_file() {
            if progress.reporter.is_none() {
                copy_file_with_options(
                    &source_path,
                    &destination_path,
                    TransferOptions {
                        overwrite: true,
                        buffer_size: options.buffer_size,
                        progress: None,
                        cancellation_token: options.cancellation_token.clone(),
                    },
                )?;
                continue;
            }

            copy_file_with_progress(
                &source_path,
                &destination_path,
                options.overwrite,
                options.buffer_size,
                progress,
            )?;
        }
    }

    Ok(())
}

fn copy_file_with_progress(
    source: &Path,
    destination: &Path,
    overwrite: bool,
    requested_buffer_size: Option<usize>,
    progress: &mut DirectoryProgress,
) -> Result<(), StorageError> {
    prepare_destination_file(destination, overwrite, true)?;
    let file_size = fs::metadata(source)
        .map_err(|err| StorageError::io("read metadata for", source, err))?
        .len();
    let buffer_size = choose_buffer_size(Some(file_size), requested_buffer_size);
    let mut source_file = open_source_file(source)?;
    let mut destination_file = open_destination_file(destination, overwrite)?;
    let mut buffer = vec![0u8; buffer_size];

    loop {
        super::common::ensure_not_cancelled(
            progress.cancellation_token.as_ref(),
            "copy directory",
        )?;
        let bytes_read = source_file
            .read(&mut buffer)
            .map_err(|err| StorageError::io("read file during directory copy", source, err))?;
        if bytes_read == 0 {
            break;
        }

        destination_file
            .write_all(&buffer[..bytes_read])
            .map_err(|err| {
                StorageError::io("write file during directory copy", destination, err)
            })?;
        progress.advance(bytes_read as u64);
    }

    Ok(())
}

struct DirectoryProgress {
    total_bytes: Option<u64>,
    bytes_transferred: u64,
    started_at: Instant,
    reporter: Option<SharedProgressReporter>,
    cancellation_token: Option<super::common::StorageCancellationToken>,
}

impl DirectoryProgress {
    fn new(
        total_bytes: Option<u64>,
        reporter: Option<SharedProgressReporter>,
        cancellation_token: Option<super::common::StorageCancellationToken>,
    ) -> Self {
        Self {
            total_bytes,
            bytes_transferred: 0,
            started_at: Instant::now(),
            reporter,
            cancellation_token,
        }
    }

    fn advance(&mut self, delta: u64) {
        self.bytes_transferred += delta;
        if let Some(reporter) = self.reporter.as_ref() {
            report_progress(
                reporter,
                self.bytes_transferred,
                self.total_bytes,
                self.started_at,
            );
        }
    }
}

fn delete_directory_recursive(
    path: &Path,
    cancellation_token: Option<&super::common::StorageCancellationToken>,
) -> Result<(), StorageError> {
    debug!(
        target: "dhara_storage::operations::directory",
        path = %path.display(),
        "walking directory for delete"
    );
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
