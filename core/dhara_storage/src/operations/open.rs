//! Engine/interop file opens with share mode and optional lock-wait.
//!
//! Path resolve belongs on [`crate::storage`] handles (or ops `normalize_*`).
//! These helpers assume a usable path and apply OS open policy only.

use std::fs::{File, OpenOptions};
use std::io;
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

use dhara_storage_core::{FileShareMode, OpenReadOptions, OpenWriteOptions};

use crate::error::StorageError;
use crate::metadata::attributes_from_path;

const LOCK_RETRY_INTERVAL: Duration = Duration::from_millis(25);

/// Open `path` for reading with the given share and optional lock-wait.
///
/// Prefer a path already resolved by [`crate::FileStorage`] / ops normalize.
///
/// # Errors
///
/// Returns I/O errors from the OS, or [`StorageError::LockTimeout`] when busy past `lock_timeout`.
pub fn open_for_read(path: impl AsRef<Path>, options: OpenReadOptions) -> Result<File, StorageError> {
    let path = path.as_ref();
    open_with_lock_wait(path, "read", options.lock_timeout, || {
        open_read_once(path, options.share)
    })
}

/// Open `path` for writing with share, create/overwrite, and optional lock-wait.
///
/// Prefer a path already resolved by [`crate::FileStorage`] / ops normalize.
///
/// # Errors
///
/// Returns I/O errors from the OS, read-only preflight failures, or
/// [`StorageError::LockTimeout`] when busy past `lock_timeout`.
pub fn open_for_write(
    path: impl AsRef<Path>,
    options: OpenWriteOptions,
) -> Result<File, StorageError> {
    let path = path.as_ref();

    if options.create_parent_directories
        && let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)
            .map_err(|err| StorageError::io("create parent directory for", parent, err))?;
    }

    // Cheap preflight: avoid waiting on a permanently read-only target.
    if path.exists() {
        let attrs = attributes_from_path(path)?;
        if attrs.read_only {
            return Err(StorageError::io(
                "open file for writing",
                path,
                io::Error::new(io::ErrorKind::PermissionDenied, "file is read-only"),
            ));
        }
    }

    open_with_lock_wait(path, "write", options.lock_timeout, || {
        open_write_once(path, options.share, options.overwrite)
    })
}

fn open_with_lock_wait(
    path: &Path,
    operation: &'static str,
    lock_timeout: Option<Duration>,
    mut attempt: impl FnMut() -> Result<File, io::Error>,
) -> Result<File, StorageError> {
    let deadline = lock_timeout.map(|timeout| Instant::now() + timeout);

    loop {
        match attempt() {
            Ok(file) => return Ok(file),
            Err(err) if is_sharing_busy(&err) => {
                match deadline {
                    None => {
                        return Err(StorageError::io(
                            if operation == "read" {
                                "open file for reading"
                            } else {
                                "open file for writing"
                            },
                            path,
                            err,
                        ));
                    }
                    Some(deadline) if Instant::now() >= deadline => {
                        return Err(StorageError::lock_timeout(path, operation));
                    }
                    Some(_) => {
                        thread::sleep(LOCK_RETRY_INTERVAL);
                    }
                }
            }
            Err(err) => {
                return Err(StorageError::io(
                    if operation == "read" {
                        "open file for reading"
                    } else {
                        "open file for writing"
                    },
                    path,
                    err,
                ));
            }
        }
    }
}

fn open_read_once(path: &Path, share: FileShareMode) -> Result<File, io::Error> {
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;

        OpenOptions::new()
            .read(true)
            .share_mode(windows_share_mode(share))
            .open(path)
    }

    #[cfg(not(windows))]
    {
        let _ = share;
        OpenOptions::new().read(true).open(path)
    }
}

fn open_write_once(path: &Path, share: FileShareMode, overwrite: bool) -> Result<File, io::Error> {
    let mut options = OpenOptions::new();
    options.write(true).create(true);

    if overwrite {
        options.truncate(true);
    } else {
        options.create_new(true);
    }

    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.share_mode(windows_share_mode(share)).open(path)
    }

    #[cfg(not(windows))]
    {
        let _ = share;
        options.open(path)
    }
}

#[cfg(windows)]
fn windows_share_mode(share: FileShareMode) -> u32 {
    const FILE_SHARE_READ: u32 = 0x0000_0001;
    const FILE_SHARE_WRITE: u32 = 0x0000_0002;
    const FILE_SHARE_DELETE: u32 = 0x0000_0004;

    match share {
        FileShareMode::Shared => FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
        FileShareMode::Exclusive => 0,
    }
}

fn is_sharing_busy(err: &io::Error) -> bool {
    #[cfg(windows)]
    {
        // ERROR_SHARING_VIOLATION = 32, ERROR_LOCK_VIOLATION = 33
        matches!(err.raw_os_error(), Some(32) | Some(33))
    }

    #[cfg(unix)]
    {
        // ETXTBSY (26 on Linux/macOS): text file busy / exclusive open conflict.
        matches!(err.raw_os_error(), Some(26)) || err.kind() == io::ErrorKind::WouldBlock
    }

    #[cfg(not(any(windows, unix)))]
    {
        err.kind() == io::ErrorKind::WouldBlock
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};

    use tempfile::tempdir;

    #[test]
    fn open_for_read_default_reads_bytes() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("a.txt");
        std::fs::write(&path, b"hello").unwrap();

        let mut file = open_for_read(&path, OpenReadOptions::default()).unwrap();
        let mut buf = String::new();
        file.read_to_string(&mut buf).unwrap();
        assert_eq!(buf, "hello");
    }

    #[test]
    fn open_for_write_overwrite_replaces() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("b.txt");
        std::fs::write(&path, b"old").unwrap();

        {
            let mut file = open_for_write(&path, OpenWriteOptions::default()).unwrap();
            file.write_all(b"new").unwrap();
        }
        assert_eq!(std::fs::read(&path).unwrap(), b"new");
    }

    #[test]
    fn open_for_write_create_new_rejects_existing() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("c.txt");
        std::fs::write(&path, b"x").unwrap();

        let err = open_for_write(
            &path,
            OpenWriteOptions {
                overwrite: false,
                ..OpenWriteOptions::default()
            },
        )
        .unwrap_err();
        assert!(matches!(err, StorageError::Io { .. }));
    }

    #[cfg(windows)]
    #[test]
    fn exclusive_write_blocks_second_open_until_timeout() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("locked.bin");
        std::fs::write(&path, b"data").unwrap();

        let _holder = open_for_write(
            &path,
            OpenWriteOptions {
                share: FileShareMode::Exclusive,
                ..OpenWriteOptions::default()
            },
        )
        .unwrap();

        let err = open_for_write(
            &path,
            OpenWriteOptions {
                share: FileShareMode::Exclusive,
                lock_timeout: Some(Duration::from_millis(80)),
                ..OpenWriteOptions::default()
            },
        )
        .unwrap_err();

        assert!(matches!(
            err,
            StorageError::LockTimeout {
                operation: "write",
                ..
            }
        ));
    }

    #[cfg(windows)]
    #[test]
    fn lock_wait_succeeds_when_holder_releases() {
        use std::sync::{Arc, Barrier};

        let dir = tempdir().unwrap();
        let path = dir.path().join("wait.bin");
        std::fs::write(&path, b"data").unwrap();
        let path_for_holder = path.clone();
        let barrier = Arc::new(Barrier::new(2));
        let barrier_holder = Arc::clone(&barrier);

        let holder = thread::spawn(move || {
            let file = open_for_write(
                &path_for_holder,
                OpenWriteOptions {
                    share: FileShareMode::Exclusive,
                    ..OpenWriteOptions::default()
                },
            )
            .unwrap();
            barrier_holder.wait();
            thread::sleep(Duration::from_millis(60));
            drop(file);
        });

        barrier.wait();
        let file = open_for_write(
            &path,
            OpenWriteOptions {
                share: FileShareMode::Exclusive,
                lock_timeout: Some(Duration::from_secs(2)),
                ..OpenWriteOptions::default()
            },
        )
        .unwrap();
        drop(file);
        holder.join().unwrap();
    }
}
