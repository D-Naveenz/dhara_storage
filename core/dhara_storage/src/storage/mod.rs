//! Path-based [`FileStorage`] and [`DirectoryStorage`] handles.
//!
//! Handles expose absolute/optional relative paths and on-demand [`crate::metadata::StorageSize`].
//! Metadata is loaded separately and is not cached on the handle.

mod directory;
mod file;

/// Path-based directory storage handle.
pub use directory::DirectoryStorage;
/// Path-based file storage handle.
pub use file::FileStorage;

use std::path::Path;

use crate::error::StorageError;
use crate::operations::common::resolve_storage_paths;

/// Controls whether directory enumeration is limited to the current folder or
/// includes nested directories recursively.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchScope {
    /// Restrict enumeration to the current directory only.
    TopDirectoryOnly,
    /// Recursively enumerate the full directory tree.
    AllDirectories,
}

/// Rust-native storage handle for an existing file-system entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StorageEntry {
    /// A file entry.
    File(FileStorage),
    /// A directory entry.
    Directory(DirectoryStorage),
}

impl StorageEntry {
    /// Resolve an existing file-system path into a typed storage handle.
    pub fn from_existing(path: impl AsRef<Path>) -> Result<Self, StorageError> {
        let resolved = resolve_storage_paths(path)?;
        if resolved.absolute.is_file() {
            return Ok(Self::File(FileStorage::from_existing(&resolved.absolute)?));
        }
        if resolved.absolute.is_dir() {
            return Ok(Self::Directory(DirectoryStorage::from_existing(
                &resolved.absolute,
            )?));
        }

        if !resolved.absolute.exists() {
            return Err(StorageError::NotFound {
                path: resolved.absolute,
            });
        }

        Err(StorageError::path_conflict(
            resolved.absolute,
            "path is neither a regular file nor a directory",
        ))
    }

    /// The absolute path represented by this handle.
    pub fn absolute_path(&self) -> &Path {
        match self {
            Self::File(file) => file.absolute_path(),
            Self::Directory(directory) => directory.absolute_path(),
        }
    }
}
