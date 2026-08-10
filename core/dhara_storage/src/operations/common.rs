//! Path utilities, locks, and buffered copy helpers for storage operations.
//!
//! Progress events, cancellation, and option bundles live in `dhara_storage_core` and
//! are re-exported from [`crate::operations`].

use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, Weak};

use once_cell::sync::Lazy;

use dhara_storage_core::{SharedProcessEventReporter, StorageCancellationToken};

use crate::error::StorageError;

use super::transfer::copy_reader_to_writer_events;

const MIN_BUFFER_SIZE: usize = 8 * 1024;
const MAX_BUFFER_SIZE: usize = 8 * 1024 * 1024;
const DEFAULT_BUFFER_SIZE: usize = 64 * 1024;

static PATH_LOCKS: Lazy<Mutex<HashMap<String, Weak<Mutex<()>>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

pub(crate) fn normalize_path(path: impl AsRef<Path>) -> Result<PathBuf, StorageError> {
    Ok(resolve_storage_paths(path)?.absolute)
}

/// Absolute path used for I/O, plus the original relative input when one was supplied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResolvedPaths {
    /// Resolved absolute path.
    pub absolute: PathBuf,
    /// Original relative path when the caller passed a relative input; otherwise `None`.
    pub relative: Option<PathBuf>,
}

/// Resolve a caller path into an absolute path, retaining a relative input when present.
pub(crate) fn resolve_storage_paths(path: impl AsRef<Path>) -> Result<ResolvedPaths, StorageError> {
    let path = path.as_ref();
    if path.is_absolute() {
        return Ok(ResolvedPaths {
            absolute: path.to_path_buf(),
            relative: None,
        });
    }

    let absolute = std::env::current_dir()
        .map(|cwd| cwd.join(path))
        .map_err(|err| StorageError::io("resolve current working directory for", path, err))?;
    Ok(ResolvedPaths {
        absolute,
        relative: Some(path.to_path_buf()),
    })
}

pub(crate) fn normalize_existing_file(path: impl AsRef<Path>) -> Result<PathBuf, StorageError> {
    let path = normalize_path(path)?;
    if !path.exists() {
        return Err(StorageError::NotFound { path });
    }
    if !path.is_file() {
        return Err(StorageError::NotAFile { path });
    }
    Ok(path)
}

pub(crate) fn normalize_existing_directory(
    path: impl AsRef<Path>,
) -> Result<PathBuf, StorageError> {
    let path = normalize_path(path)?;
    if !path.exists() {
        return Err(StorageError::NotFound { path });
    }
    if !path.is_dir() {
        return Err(StorageError::NotADirectory { path });
    }
    Ok(path)
}

pub(crate) fn ensure_parent_directory(
    path: &Path,
    create: bool,
) -> Result<Option<PathBuf>, StorageError> {
    let Some(parent) = path.parent() else {
        return Ok(None);
    };

    if parent.as_os_str().is_empty() {
        return Ok(None);
    }

    if create {
        fs::create_dir_all(parent)
            .map_err(|err| StorageError::io("create parent directory for", parent, err))?;
    } else if !parent.exists() {
        return Err(StorageError::NotFound {
            path: parent.to_path_buf(),
        });
    }

    Ok(Some(parent.to_path_buf()))
}

pub(crate) fn prepare_destination_file(
    destination: &Path,
    overwrite: bool,
    create_parent_directories: bool,
) -> Result<(), StorageError> {
    ensure_parent_directory(destination, create_parent_directories)?;

    if destination.exists() {
        if destination.is_dir() {
            return Err(StorageError::path_conflict(
                destination.to_path_buf(),
                "destination points to a directory",
            ));
        }

        if !overwrite {
            return Err(StorageError::already_exists(destination.to_path_buf()));
        }
    }

    Ok(())
}

pub(crate) fn prepare_destination_directory(
    destination: &Path,
    overwrite: bool,
) -> Result<(), StorageError> {
    ensure_parent_directory(destination, true)?;

    if destination.exists() {
        if destination.is_file() {
            return Err(StorageError::path_conflict(
                destination.to_path_buf(),
                "destination points to a file",
            ));
        }

        if !overwrite {
            return Err(StorageError::already_exists(destination.to_path_buf()));
        }
    }

    Ok(())
}

pub(crate) fn validate_single_path_name(
    value: &str,
    kind: &'static str,
) -> Result<(), StorageError> {
    if value.trim().is_empty() {
        return Err(StorageError::invalid_name(kind, value));
    }

    if value == "." || value == ".." {
        return Err(StorageError::invalid_name(kind, value));
    }

    let path = Path::new(value);
    if path.components().count() != 1 {
        return Err(StorageError::invalid_name(kind, value));
    }

    #[cfg(windows)]
    {
        const INVALID: [char; 9] = ['<', '>', ':', '"', '/', '\\', '|', '?', '*'];
        if value.chars().any(|ch| INVALID.contains(&ch)) {
            return Err(StorageError::invalid_name(kind, value));
        }
    }

    Ok(())
}

pub(crate) fn same_volume(left: &Path, right: &Path) -> bool {
    #[cfg(windows)]
    {
        use std::path::Component;

        let left_prefix = left.components().find_map(|component| match component {
            Component::Prefix(prefix) => Some(prefix.as_os_str().to_string_lossy().to_string()),
            _ => None,
        });
        let right_prefix = right.components().find_map(|component| match component {
            Component::Prefix(prefix) => Some(prefix.as_os_str().to_string_lossy().to_string()),
            _ => None,
        });

        left_prefix == right_prefix
    }

    #[cfg(not(windows))]
    {
        let _ = (left, right);
        true
    }
}

pub(crate) fn choose_buffer_size(total_bytes: Option<u64>, requested: Option<usize>) -> usize {
    if let Some(requested) = requested {
        return requested.clamp(MIN_BUFFER_SIZE, MAX_BUFFER_SIZE);
    }

    match total_bytes {
        Some(0) => MIN_BUFFER_SIZE,
        Some(total) => ((total / 128) as usize).clamp(MIN_BUFFER_SIZE, MAX_BUFFER_SIZE),
        None => DEFAULT_BUFFER_SIZE,
    }
}

pub(crate) fn copy_reader_to_writer<R, W>(
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
    copy_reader_to_writer_events(
        reader,
        writer,
        total_bytes,
        buffer_size,
        progress,
        cancellation_token,
        None,
        operation,
    )
}

pub(crate) fn ensure_not_cancelled(
    cancellation_token: Option<&StorageCancellationToken>,
    operation: &'static str,
) -> Result<(), StorageError> {
    if cancellation_token.is_some_and(StorageCancellationToken::is_cancelled) {
        return Err(StorageError::cancelled(operation));
    }

    Ok(())
}

pub(crate) fn open_source_file(path: &Path) -> Result<File, StorageError> {
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;

        const FILE_SHARE_READ: u32 = 0x0000_0001;
        const FILE_SHARE_WRITE: u32 = 0x0000_0002;
        const FILE_SHARE_DELETE: u32 = 0x0000_0004;

        OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE)
            .open(path)
            .map_err(|err| StorageError::io("open file for reading", path, err))
    }

    #[cfg(not(windows))]
    {
        OpenOptions::new()
            .read(true)
            .open(path)
            .map_err(|err| StorageError::io("open file for reading", path, err))
    }
}

pub(crate) fn open_destination_file(path: &Path, overwrite: bool) -> Result<File, StorageError> {
    let mut options = OpenOptions::new();
    options.write(true).create(true);

    if overwrite {
        options.truncate(true);
    } else {
        options.create_new(true);
    }

    options
        .open(path)
        .map_err(|err| StorageError::io("open file for writing", path, err))
}

pub(crate) fn lock_write_targets<T>(
    paths: &[&Path],
    operation: impl FnOnce() -> Result<T, StorageError>,
) -> Result<T, StorageError> {
    let mut keys = paths
        .iter()
        .flat_map(|path| immediate_lock_keys(path))
        .collect::<Vec<_>>();
    keys.sort();
    keys.dedup();

    let locks = {
        let mut registry = PATH_LOCKS
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        keys.iter()
            .map(|key| {
                if let Some(existing) = registry.get(key).and_then(Weak::upgrade) {
                    return existing;
                }

                let created = Arc::new(Mutex::new(()));
                registry.insert(key.clone(), Arc::downgrade(&created));
                created
            })
            .collect::<Vec<_>>()
    };

    let _guards = locks
        .iter()
        .map(|lock| lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner()))
        .collect::<Vec<_>>();

    operation()
}

fn immediate_lock_keys(path: &Path) -> Vec<String> {
    let mut keys = vec![path_lock_key(path)];
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        keys.push(path_lock_key(parent));
    }
    keys
}

fn path_lock_key(path: &Path) -> String {
    #[cfg(windows)]
    {
        path.to_string_lossy().to_ascii_lowercase()
    }

    #[cfg(not(windows))]
    {
        path.to_string_lossy().into_owned()
    }
}
