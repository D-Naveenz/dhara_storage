use std::io::Cursor;
use std::path::{Path, PathBuf};

use tokio::task;

use crate::error::StorageError;

use super::common::{DirectoryDeleteOptions, ReadOptions, TransferOptions, WriteOptions};

/// Async wrapper for [`super::copy_file`]. Runs on Tokio's blocking pool.
pub async fn copy_file_async(
    source: impl AsRef<Path>,
    destination: impl AsRef<Path>,
    options: TransferOptions,
) -> Result<PathBuf, StorageError> {
    let source = source.as_ref().to_path_buf();
    let destination = destination.as_ref().to_path_buf();
    run_blocking("copy file", move || {
        super::copy_file_with_options(source, destination, options)
    })
    .await
}

/// Async wrapper for [`super::move_file`]. Runs on Tokio's blocking pool.
pub async fn move_file_async(
    source: impl AsRef<Path>,
    destination: impl AsRef<Path>,
    options: TransferOptions,
) -> Result<PathBuf, StorageError> {
    let source = source.as_ref().to_path_buf();
    let destination = destination.as_ref().to_path_buf();
    run_blocking("move file", move || {
        super::move_file_with_options(source, destination, options)
    })
    .await
}

/// Async wrapper for [`super::rename_file`]. Runs on Tokio's blocking pool.
pub async fn rename_file_async(
    source: impl AsRef<Path>,
    new_name: impl Into<String>,
) -> Result<PathBuf, StorageError> {
    let source = source.as_ref().to_path_buf();
    let new_name = new_name.into();
    run_blocking("rename file", move || super::rename_file(source, &new_name)).await
}

/// Async wrapper for [`super::delete_file`]. Runs on Tokio's blocking pool.
pub async fn delete_file_async(path: impl AsRef<Path>) -> Result<(), StorageError> {
    let path = path.as_ref().to_path_buf();
    run_blocking("delete file", move || super::delete_file(path)).await
}

/// Async wrapper for [`super::read_file`]. Runs on Tokio's blocking pool.
pub async fn read_file_async(
    path: impl AsRef<Path>,
    options: ReadOptions,
) -> Result<Vec<u8>, StorageError> {
    let path = path.as_ref().to_path_buf();
    run_blocking("read file", move || {
        super::file::read_file_with_options(path, options)
    })
    .await
}

/// Async wrapper for [`super::read_file_to_string`]. Runs on Tokio's blocking pool.
pub async fn read_file_to_string_async(path: impl AsRef<Path>) -> Result<String, StorageError> {
    let path = path.as_ref().to_path_buf();
    run_blocking("read file as text", move || {
        super::read_file_to_string(path)
    })
    .await
}

/// Async wrapper for [`super::write_file`]. Runs on Tokio's blocking pool.
pub async fn write_file_async(
    path: impl AsRef<Path>,
    bytes: impl AsRef<[u8]>,
    options: WriteOptions,
) -> Result<PathBuf, StorageError> {
    let path = path.as_ref().to_path_buf();
    let bytes = bytes.as_ref().to_vec();
    run_blocking("write file", move || {
        let mut cursor = Cursor::new(bytes);
        super::write_file_from_reader(path, &mut cursor, options)
    })
    .await
}

/// Async wrapper for [`super::write_file_string`]. Runs on Tokio's blocking pool.
pub async fn write_file_string_async(
    path: impl AsRef<Path>,
    text: impl Into<String>,
    options: WriteOptions,
) -> Result<PathBuf, StorageError> {
    let path = path.as_ref().to_path_buf();
    let text = text.into();
    write_file_async(path, text.into_bytes(), options).await
}

/// Async wrapper for byte-slice backed "reader" writes. Runs on Tokio's blocking pool.
pub async fn write_file_from_reader_async(
    path: impl AsRef<Path>,
    bytes: Vec<u8>,
    options: WriteOptions,
) -> Result<PathBuf, StorageError> {
    let path = path.as_ref().to_path_buf();
    run_blocking("write file from reader", move || {
        let mut cursor = Cursor::new(bytes);
        super::write_file_from_reader(path, &mut cursor, options)
    })
    .await
}

/// Async wrapper for [`super::create_directory`]. Runs on Tokio's blocking pool.
pub async fn create_directory_async(path: impl AsRef<Path>) -> Result<PathBuf, StorageError> {
    let path = path.as_ref().to_path_buf();
    run_blocking("create directory", move || super::create_directory(path)).await
}

/// Async wrapper for [`super::create_directory_all`]. Runs on Tokio's blocking pool.
pub async fn create_directory_all_async(path: impl AsRef<Path>) -> Result<PathBuf, StorageError> {
    let path = path.as_ref().to_path_buf();
    run_blocking("create directory tree", move || {
        super::create_directory_all(path)
    })
    .await
}

/// Async wrapper for [`super::copy_directory`]. Runs on Tokio's blocking pool.
pub async fn copy_directory_async(
    source: impl AsRef<Path>,
    destination: impl AsRef<Path>,
    options: TransferOptions,
) -> Result<PathBuf, StorageError> {
    let source = source.as_ref().to_path_buf();
    let destination = destination.as_ref().to_path_buf();
    run_blocking("copy directory", move || {
        super::copy_directory_with_options(source, destination, options)
    })
    .await
}

/// Async wrapper for [`super::move_directory`]. Runs on Tokio's blocking pool.
pub async fn move_directory_async(
    source: impl AsRef<Path>,
    destination: impl AsRef<Path>,
    options: TransferOptions,
) -> Result<PathBuf, StorageError> {
    let source = source.as_ref().to_path_buf();
    let destination = destination.as_ref().to_path_buf();
    run_blocking("move directory", move || {
        super::move_directory_with_options(source, destination, options)
    })
    .await
}

/// Async wrapper for [`super::rename_directory`]. Runs on Tokio's blocking pool.
pub async fn rename_directory_async(
    source: impl AsRef<Path>,
    new_name: impl Into<String>,
) -> Result<PathBuf, StorageError> {
    let source = source.as_ref().to_path_buf();
    let new_name = new_name.into();
    run_blocking("rename directory", move || {
        super::rename_directory(source, &new_name)
    })
    .await
}

/// Async wrapper for [`super::delete_directory`]. Runs on Tokio's blocking pool.
pub async fn delete_directory_async(
    path: impl AsRef<Path>,
    options: DirectoryDeleteOptions,
) -> Result<(), StorageError> {
    let path = path.as_ref().to_path_buf();
    run_blocking("delete directory", move || {
        super::delete_directory_with_options(path, options)
    })
    .await
}

async fn run_blocking<T>(
    operation: &'static str,
    task_fn: impl FnOnce() -> Result<T, StorageError> + Send + 'static,
) -> Result<T, StorageError>
where
    T: Send + 'static,
{
    task::spawn_blocking(task_fn)
        .await
        .map_err(|err| StorageError::async_runtime(operation, err.to_string()))?
}
