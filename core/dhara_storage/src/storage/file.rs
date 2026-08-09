//! [`FileStorage`] — path-based file handle for I/O, transfers, and metadata.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::error::StorageError;
use crate::metadata::{
    FileMetadata, StorageAttributes, StoragePermissions, StorageSize, is_temporary_path,
};
use crate::operations::common::{ResolvedPaths, resolve_storage_paths};
use crate::operations::{
    ReadOptions, TransferOptions, WriteOptions, copy_file, copy_file_with_options, delete_file,
    move_file, move_file_with_options, read_file, read_file_to_string, rename_file, write_file,
    write_file_from_reader, write_file_string,
};

/// Rust-native handle for file operations and on-demand metadata.
///
/// Paths and [`Self::size`] live on the handle. Metadata is loaded via
/// [`Self::metadata`] and is not cached here. Content analysis is
/// [`FileMetadata::analyze`] on a held metadata value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileStorage {
    absolute_path: PathBuf,
    relative_path: Option<PathBuf>,
}

impl FileStorage {
    fn from_resolved(resolved: ResolvedPaths) -> Self {
        Self {
            absolute_path: resolved.absolute,
            relative_path: resolved.relative,
        }
    }

    fn from_absolute(absolute: PathBuf) -> Self {
        Self {
            absolute_path: absolute,
            relative_path: None,
        }
    }

    fn from_destination(destination: &Path, absolute: PathBuf) -> Self {
        Self {
            absolute_path: absolute,
            relative_path: if destination.is_absolute() {
                None
            } else {
                Some(destination.to_path_buf())
            },
        }
    }

    /// Create a path-based file handle without requiring the file to exist yet.
    pub fn new(path: impl AsRef<Path>) -> Result<Self, StorageError> {
        Ok(Self::from_resolved(resolve_storage_paths(path)?))
    }

    /// Create a file handle for an existing file.
    pub fn from_existing(path: impl AsRef<Path>) -> Result<Self, StorageError> {
        let resolved = resolve_storage_paths(path)?;
        if !resolved.absolute.exists() {
            return Err(StorageError::NotFound {
                path: resolved.absolute,
            });
        }
        if !resolved.absolute.is_file() {
            return Err(StorageError::NotAFile {
                path: resolved.absolute,
            });
        }
        Ok(Self::from_resolved(resolved))
    }

    /// Create a named temporary file and return a handle for it.
    ///
    /// Delegates to the `tempfile` crate for portable TEMP / `O_TMPFILE` / `$TMPDIR` behavior.
    pub fn create_temporary() -> Result<Self, StorageError> {
        let file = tempfile::NamedTempFile::new().map_err(|err| {
            StorageError::io("create temporary file in", std::env::temp_dir(), err)
        })?;
        let path = file.into_temp_path().keep().map_err(|err| {
            StorageError::io(
                "persist temporary file path in",
                std::env::temp_dir(),
                err.into(),
            )
        })?;
        Self::from_existing(path)
    }

    /// Resolved absolute path used for I/O.
    pub fn absolute_path(&self) -> &Path {
        &self.absolute_path
    }

    /// Original relative path when this handle was initialized with a relative input.
    pub fn relative_path(&self) -> Option<&Path> {
        self.relative_path.as_deref()
    }

    /// File name including extension.
    pub fn name(&self) -> Option<&str> {
        self.absolute_path
            .file_name()
            .and_then(|value| value.to_str())
    }

    /// Measure current file size on the spot (not cached).
    pub fn size(&self) -> Result<StorageSize, StorageError> {
        let metadata = fs::metadata(&self.absolute_path)
            .map_err(|err| StorageError::io("read metadata for", &self.absolute_path, err))?;
        Ok(StorageSize::from_bytes(metadata.len()))
    }

    /// Load on-demand file metadata (no analysis until [`FileMetadata::analyze`]).
    pub fn metadata(&self) -> Result<FileMetadata, StorageError> {
        FileMetadata::load(&self.absolute_path)
    }

    /// Read settable attributes for this file.
    pub fn attributes(&self) -> Result<StorageAttributes, StorageError> {
        StorageAttributes::from_path(&self.absolute_path)
    }

    /// Apply settable attributes to this file.
    pub fn set_attributes(&self, attributes: StorageAttributes) -> Result<(), StorageError> {
        attributes.apply_to(&self.absolute_path)
    }

    /// Effective permissions for the current process.
    pub fn permissions(&self) -> Result<StoragePermissions, StorageError> {
        StoragePermissions::from_path(&self.absolute_path)
    }

    /// Whether this path looks temporary by attribute and/or temp location.
    pub fn is_temporary(&self) -> Result<bool, StorageError> {
        is_temporary_path(&self.absolute_path)
    }

    /// Read the full file into memory.
    pub fn read(&self) -> Result<Vec<u8>, StorageError> {
        read_file(&self.absolute_path)
    }

    /// Read the full file as UTF-8 text.
    pub fn read_to_string(&self) -> Result<String, StorageError> {
        read_file_to_string(&self.absolute_path)
    }

    /// Read the full file into memory with progress reporting.
    pub fn read_with_options(&self, options: ReadOptions) -> Result<Vec<u8>, StorageError> {
        crate::operations::file::read_file_with_options(&self.absolute_path, options)
    }

    /// Write raw bytes to the file.
    pub fn write(&self, bytes: impl AsRef<[u8]>) -> Result<Self, StorageError> {
        let path = write_file(&self.absolute_path, bytes)?;
        Ok(Self::from_absolute(path))
    }

    /// Write UTF-8 text to the file.
    pub fn write_string(&self, text: impl AsRef<str>) -> Result<Self, StorageError> {
        let path = write_file_string(&self.absolute_path, text)?;
        Ok(Self::from_absolute(path))
    }

    /// Stream bytes into the file using the supplied options.
    pub fn write_from_reader(
        &self,
        reader: &mut impl Read,
        options: WriteOptions,
    ) -> Result<Self, StorageError> {
        let path = write_file_from_reader(&self.absolute_path, reader, options)?;
        Ok(Self::from_absolute(path))
    }

    /// Copy the file to an exact destination path.
    pub fn copy_to(&self, destination: impl AsRef<Path>) -> Result<Self, StorageError> {
        let destination = destination.as_ref();
        let path = copy_file(&self.absolute_path, destination)?;
        Ok(Self::from_destination(destination, path))
    }

    /// Copy the file with overwrite and progress control.
    pub fn copy_to_with_options(
        &self,
        destination: impl AsRef<Path>,
        options: TransferOptions,
    ) -> Result<Self, StorageError> {
        let destination = destination.as_ref();
        let path = copy_file_with_options(&self.absolute_path, destination, options)?;
        Ok(Self::from_destination(destination, path))
    }

    /// Move the file to an exact destination path.
    pub fn move_to(&self, destination: impl AsRef<Path>) -> Result<Self, StorageError> {
        let destination = destination.as_ref();
        let path = move_file(&self.absolute_path, destination)?;
        Ok(Self::from_destination(destination, path))
    }

    /// Move the file with overwrite and progress control.
    pub fn move_to_with_options(
        &self,
        destination: impl AsRef<Path>,
        options: TransferOptions,
    ) -> Result<Self, StorageError> {
        let destination = destination.as_ref();
        let path = move_file_with_options(&self.absolute_path, destination, options)?;
        Ok(Self::from_destination(destination, path))
    }

    /// Rename the file inside its current parent directory.
    pub fn rename(&self, new_name: &str) -> Result<Self, StorageError> {
        let path = rename_file(&self.absolute_path, new_name)?;
        Ok(Self::from_absolute(path))
    }

    /// Delete the file represented by this handle.
    pub fn delete(&self) -> Result<(), StorageError> {
        delete_file(&self.absolute_path)
    }
}

#[cfg(feature = "async-tokio")]
impl FileStorage {
    /// Async variant of [`Self::copy_to_with_options`].
    pub async fn copy_to_async(
        &self,
        destination: impl AsRef<Path>,
        options: TransferOptions,
    ) -> Result<Self, StorageError> {
        let destination = destination.as_ref();
        let path =
            crate::operations::copy_file_async(&self.absolute_path, destination, options).await?;
        Ok(Self::from_destination(destination, path))
    }

    /// Async variant of [`Self::move_to_with_options`].
    pub async fn move_to_async(
        &self,
        destination: impl AsRef<Path>,
        options: TransferOptions,
    ) -> Result<Self, StorageError> {
        let destination = destination.as_ref();
        let path =
            crate::operations::move_file_async(&self.absolute_path, destination, options).await?;
        Ok(Self::from_destination(destination, path))
    }

    /// Async variant of [`Self::rename`].
    pub async fn rename_async(&self, new_name: impl Into<String>) -> Result<Self, StorageError> {
        let path = crate::operations::rename_file_async(&self.absolute_path, new_name).await?;
        Ok(Self::from_absolute(path))
    }

    /// Async variant of [`Self::delete`].
    pub async fn delete_async(&self) -> Result<(), StorageError> {
        crate::operations::delete_file_async(&self.absolute_path).await
    }

    /// Async variant of [`Self::read_with_options`].
    pub async fn read_async(&self, options: ReadOptions) -> Result<Vec<u8>, StorageError> {
        crate::operations::read_file_async(&self.absolute_path, options).await
    }

    /// Async variant of [`Self::read_to_string`].
    pub async fn read_to_string_async(&self) -> Result<String, StorageError> {
        crate::operations::read_file_to_string_async(&self.absolute_path).await
    }

    /// Async variant of [`Self::write_from_reader`], backed by a byte buffer.
    pub async fn write_async(
        &self,
        bytes: impl AsRef<[u8]>,
        options: WriteOptions,
    ) -> Result<Self, StorageError> {
        let path =
            crate::operations::write_file_async(&self.absolute_path, bytes, options).await?;
        Ok(Self::from_absolute(path))
    }

    /// Async variant of [`Self::write_string`].
    pub async fn write_string_async(
        &self,
        text: impl Into<String>,
        options: WriteOptions,
    ) -> Result<Self, StorageError> {
        let path =
            crate::operations::write_file_string_async(&self.absolute_path, text, options).await?;
        Ok(Self::from_absolute(path))
    }
}
