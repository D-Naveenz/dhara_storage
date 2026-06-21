use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, Weak};
use std::time::Instant;

use once_cell::sync::Lazy;

use crate::error::StorageError;

const MIN_BUFFER_SIZE: usize = 8 * 1024;
const MAX_BUFFER_SIZE: usize = 8 * 1024 * 1024;
const DEFAULT_BUFFER_SIZE: usize = 64 * 1024;

static PATH_LOCKS: Lazy<Mutex<HashMap<String, Weak<Mutex<()>>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

/// Progress details emitted by long-running storage operations.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StorageProgress {
    /// Total number of bytes expected, when known in advance.
    pub total_bytes: Option<u64>,
    /// Number of bytes transferred so far.
    pub bytes_transferred: u64,
    /// Best-effort average transfer speed in bytes per second.
    pub bytes_per_second: f64,
}

/// Cooperative cancellation token for long-running storage operations.
#[derive(Debug, Clone, Default)]
pub struct StorageCancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl StorageCancellationToken {
    /// Create a new uncancelled token.
    pub fn new() -> Self {
        Self::default()
    }

    /// Mark the token as cancelled.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    /// Returns true when cancellation has been requested.
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

/// Callback interface for optional storage progress reporting.
pub trait ProgressReporter: Send + Sync + 'static {
    /// Receives a progress snapshot from an in-flight storage operation.
    fn report(&self, progress: StorageProgress);
}

impl<F> ProgressReporter for F
where
    F: Fn(StorageProgress) + Send + Sync + 'static,
{
    fn report(&self, progress: StorageProgress) {
        self(progress);
    }
}

/// Shared reporter type used by both sync and async APIs.
pub type SharedProgressReporter = Arc<dyn ProgressReporter>;

/// Common options for copy and move style operations.
#[derive(Clone, Default)]
pub struct TransferOptions {
    /// Replace the destination when it already exists.
    pub overwrite: bool,
    /// Override the buffered copy size when progress reporting is enabled.
    pub buffer_size: Option<usize>,
    /// Optional progress callback. When omitted, the fastest available path is used.
    pub progress: Option<SharedProgressReporter>,
    /// Optional cancellation token for cooperative cancellation.
    pub cancellation_token: Option<StorageCancellationToken>,
}

/// Common options for byte-oriented read operations.
#[derive(Clone, Default)]
pub struct ReadOptions {
    /// Override the buffered read size when progress reporting is enabled.
    pub buffer_size: Option<usize>,
    /// Optional progress callback. When omitted, the fastest available path is used.
    pub progress: Option<SharedProgressReporter>,
    /// Optional cancellation token for cooperative cancellation.
    pub cancellation_token: Option<StorageCancellationToken>,
}

/// Common options for file write operations.
#[derive(Clone)]
pub struct WriteOptions {
    /// Replace the destination when it already exists.
    pub overwrite: bool,
    /// Create missing parent directories before opening the destination.
    pub create_parent_directories: bool,
    /// Override the buffered copy size when progress reporting is enabled.
    pub buffer_size: Option<usize>,
    /// Optional progress callback. When omitted, the fastest available path is used.
    pub progress: Option<SharedProgressReporter>,
    /// Optional cancellation token for cooperative cancellation.
    pub cancellation_token: Option<StorageCancellationToken>,
}

impl Default for WriteOptions {
    fn default() -> Self {
        Self {
            overwrite: true,
            create_parent_directories: true,
            buffer_size: None,
            progress: None,
            cancellation_token: None,
        }
    }
}

/// Options for directory deletion.
#[derive(Debug, Clone)]
pub struct DirectoryDeleteOptions {
    /// Delete the directory recursively when true.
    pub recursive: bool,
    /// Optional cancellation token for cooperative cancellation.
    pub cancellation_token: Option<StorageCancellationToken>,
}

impl Default for DirectoryDeleteOptions {
    fn default() -> Self {
        Self {
            recursive: true,
            cancellation_token: None,
        }
    }
}

pub(crate) fn normalize_path(path: impl AsRef<Path>) -> Result<PathBuf, StorageError> {
    let path = path.as_ref();
    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }

    std::env::current_dir()
        .map(|cwd| cwd.join(path))
        .map_err(|err| StorageError::io("resolve current working directory for", path, err))
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
    progress: Option<&SharedProgressReporter>,
    cancellation_token: Option<&StorageCancellationToken>,
    operation: &'static str,
) -> Result<u64, StorageError>
where
    R: Read,
    W: Write,
{
    if progress.is_none() && cancellation_token.is_none() {
        return io::copy(reader, writer).map_err(|err| StorageError::reader_io("copy from", err));
    }

    let mut buffer = vec![0u8; buffer_size];
    let mut transferred = 0u64;
    let start = Instant::now();

    loop {
        ensure_not_cancelled(cancellation_token, operation)?;
        let read = reader
            .read(&mut buffer)
            .map_err(|err| StorageError::reader_io("read from", err))?;
        if read == 0 {
            break;
        }

        writer
            .write_all(&buffer[..read])
            .map_err(|err| StorageError::reader_io("write to", err))?;
        transferred += read as u64;

        if let Some(progress) = progress {
            report_progress(progress, transferred, total_bytes, start);
        }
    }

    Ok(transferred)
}

pub(crate) fn report_progress(
    progress: &SharedProgressReporter,
    bytes_transferred: u64,
    total_bytes: Option<u64>,
    started_at: Instant,
) {
    let elapsed = started_at.elapsed().as_secs_f64();
    let bytes_per_second = if elapsed > 0.0 {
        bytes_transferred as f64 / elapsed
    } else {
        0.0
    };

    progress.report(StorageProgress {
        total_bytes,
        bytes_transferred,
        bytes_per_second,
    });
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
