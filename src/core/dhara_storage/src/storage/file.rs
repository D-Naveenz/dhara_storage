use std::io::Read;
use std::path::{Path, PathBuf};

use crate::error::StorageError;
use crate::info::FileInfo;
use crate::operations::common::normalize_path;
use crate::operations::{
    ReadOptions, TransferOptions, WriteOptions, copy_file, copy_file_with_options, delete_file,
    move_file, move_file_with_options, read_file, read_file_to_string, rename_file, write_file,
    write_file_from_reader, write_file_string,
};

/// Rust-native handle for file operations and metadata lookups.
///
/// The handle itself is lightweight and path-based. Expensive metadata and content
/// analysis remain opt-in through [`FileInfo`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileStorage {
    path: PathBuf,
}

impl FileStorage {
    /// Create a path-based file handle without touching the file system.
    pub fn new(path: impl AsRef<Path>) -> Result<Self, StorageError> {
        Ok(Self {
            path: normalize_path(path)?,
        })
    }

    /// Create a file handle for an existing file.
    pub fn from_existing(path: impl AsRef<Path>) -> Result<Self, StorageError> {
        let path = normalize_path(path)?;
        if !path.exists() {
            return Err(StorageError::NotFound { path });
        }
        if !path.is_file() {
            return Err(StorageError::NotAFile { path });
        }

        Ok(Self { path })
    }

    /// Absolute path represented by this handle.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// File name including extension.
    pub fn name(&self) -> Option<&str> {
        self.path.file_name().and_then(|value| value.to_str())
    }

    /// Load cheap file metadata without running content analysis.
    pub fn info(&self) -> Result<FileInfo, StorageError> {
        FileInfo::from_path(&self.path)
    }

    /// Load file metadata and precompute content analysis in parallel.
    pub fn info_with_analysis(&self) -> Result<FileInfo, StorageError> {
        FileInfo::from_path_with_analysis(&self.path)
    }

    /// Read the full file into memory.
    pub fn read(&self) -> Result<Vec<u8>, StorageError> {
        read_file(&self.path)
    }

    /// Read the full file as UTF-8 text.
    pub fn read_to_string(&self) -> Result<String, StorageError> {
        read_file_to_string(&self.path)
    }

    /// Read the full file into memory with progress reporting.
    pub fn read_with_options(&self, options: ReadOptions) -> Result<Vec<u8>, StorageError> {
        crate::operations::file::read_file_with_options(&self.path, options)
    }

    /// Write raw bytes to the file.
    pub fn write(&self, bytes: impl AsRef<[u8]>) -> Result<Self, StorageError> {
        let path = write_file(&self.path, bytes)?;
        Ok(Self { path })
    }

    /// Write UTF-8 text to the file.
    pub fn write_string(&self, text: impl AsRef<str>) -> Result<Self, StorageError> {
        let path = write_file_string(&self.path, text)?;
        Ok(Self { path })
    }

    /// Stream bytes into the file using the supplied options.
    pub fn write_from_reader(
        &self,
        reader: &mut impl Read,
        options: WriteOptions,
    ) -> Result<Self, StorageError> {
        let path = write_file_from_reader(&self.path, reader, options)?;
        Ok(Self { path })
    }

    /// Copy the file to an exact destination path.
    pub fn copy_to(&self, destination: impl AsRef<Path>) -> Result<Self, StorageError> {
        let path = copy_file(&self.path, destination)?;
        Ok(Self { path })
    }

    /// Copy the file with overwrite and progress control.
    pub fn copy_to_with_options(
        &self,
        destination: impl AsRef<Path>,
        options: TransferOptions,
    ) -> Result<Self, StorageError> {
        let path = copy_file_with_options(&self.path, destination, options)?;
        Ok(Self { path })
    }

    /// Move the file to an exact destination path.
    ///
    /// The original handle is not mutated; a new handle for the destination path is returned.
    pub fn move_to(&self, destination: impl AsRef<Path>) -> Result<Self, StorageError> {
        let path = move_file(&self.path, destination)?;
        Ok(Self { path })
    }

    /// Move the file with overwrite and progress control.
    pub fn move_to_with_options(
        &self,
        destination: impl AsRef<Path>,
        options: TransferOptions,
    ) -> Result<Self, StorageError> {
        let path = move_file_with_options(&self.path, destination, options)?;
        Ok(Self { path })
    }

    /// Rename the file inside its current parent directory.
    ///
    /// The original handle is not mutated; a new handle for the renamed path is returned.
    pub fn rename(&self, new_name: &str) -> Result<Self, StorageError> {
        let path = rename_file(&self.path, new_name)?;
        Ok(Self { path })
    }

    /// Delete the file represented by this handle.
    pub fn delete(&self) -> Result<(), StorageError> {
        delete_file(&self.path)
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
        let path = crate::operations::copy_file_async(&self.path, destination, options).await?;
        Ok(Self { path })
    }

    /// Async variant of [`Self::move_to_with_options`].
    pub async fn move_to_async(
        &self,
        destination: impl AsRef<Path>,
        options: TransferOptions,
    ) -> Result<Self, StorageError> {
        let path = crate::operations::move_file_async(&self.path, destination, options).await?;
        Ok(Self { path })
    }

    /// Async variant of [`Self::rename`].
    pub async fn rename_async(&self, new_name: impl Into<String>) -> Result<Self, StorageError> {
        let path = crate::operations::rename_file_async(&self.path, new_name).await?;
        Ok(Self { path })
    }

    /// Async variant of [`Self::delete`].
    pub async fn delete_async(&self) -> Result<(), StorageError> {
        crate::operations::delete_file_async(&self.path).await
    }

    /// Async variant of [`Self::read_with_options`].
    pub async fn read_async(&self, options: ReadOptions) -> Result<Vec<u8>, StorageError> {
        crate::operations::read_file_async(&self.path, options).await
    }

    /// Async variant of [`Self::read_to_string`].
    pub async fn read_to_string_async(&self) -> Result<String, StorageError> {
        crate::operations::read_file_to_string_async(&self.path).await
    }

    /// Async variant of [`Self::write_from_reader`], backed by a byte buffer.
    pub async fn write_async(
        &self,
        bytes: impl AsRef<[u8]>,
        options: WriteOptions,
    ) -> Result<Self, StorageError> {
        let path = crate::operations::write_file_async(&self.path, bytes, options).await?;
        Ok(Self { path })
    }

    /// Async variant of [`Self::write_string`].
    pub async fn write_string_async(
        &self,
        text: impl Into<String>,
        options: WriteOptions,
    ) -> Result<Self, StorageError> {
        let path = crate::operations::write_file_string_async(&self.path, text, options).await?;
        Ok(Self { path })
    }
}
