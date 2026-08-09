//! [`DirectoryStorage`] — path-based directory handle for enumeration, transfers, and watching.

use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::error::StorageError;
use crate::metadata::{
    DirectoryMetadata, DirectorySummary, StorageAttributes, StoragePermissions, StorageSize,
    is_temporary_path, scan_directory_summary,
};
use crate::operations::common::{ResolvedPaths, resolve_storage_paths};
use crate::operations::{
    DirectoryDeleteOptions, TransferOptions, copy_directory, copy_directory_with_options,
    create_directory, create_directory_all, delete_directory, delete_directory_with_options,
    move_directory, move_directory_with_options, rename_directory,
};
use crate::watch::{DirectoryWatchHandle, StorageWatchConfig};

use super::{FileStorage, SearchScope, StorageEntry};

/// Rust-native handle for directory operations and on-demand metadata.
///
/// Paths and [`Self::size`] live on the handle. Recursive size walks block until
/// complete when measured. Metadata is loaded via [`Self::metadata`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectoryStorage {
    absolute_path: PathBuf,
    relative_path: Option<PathBuf>,
}

impl DirectoryStorage {
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

    /// Create a path-based directory handle without requiring the directory to exist yet.
    pub fn new(path: impl AsRef<Path>) -> Result<Self, StorageError> {
        Ok(Self::from_resolved(resolve_storage_paths(path)?))
    }

    /// Create a directory handle for an existing directory.
    pub fn from_existing(path: impl AsRef<Path>) -> Result<Self, StorageError> {
        let resolved = resolve_storage_paths(path)?;
        if !resolved.absolute.exists() {
            return Err(StorageError::NotFound {
                path: resolved.absolute,
            });
        }
        if !resolved.absolute.is_dir() {
            return Err(StorageError::NotADirectory {
                path: resolved.absolute,
            });
        }
        Ok(Self::from_resolved(resolved))
    }

    /// Create a temporary directory and return a handle for it.
    pub fn create_temporary() -> Result<Self, StorageError> {
        let dir = tempfile::TempDir::new().map_err(|err| {
            StorageError::io("create temporary directory in", std::env::temp_dir(), err)
        })?;
        let path = dir.keep();
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

    /// Directory name.
    pub fn name(&self) -> Option<&str> {
        self.absolute_path
            .file_name()
            .and_then(|value| value.to_str())
    }

    /// Measure recursive directory size on the spot (blocks until the walk finishes).
    pub fn size(&self) -> Result<StorageSize, StorageError> {
        Ok(scan_directory_summary(&self.absolute_path)?.to_storage_size())
    }

    /// Recursive summary (size + counts); blocks until complete.
    pub fn summary(&self) -> Result<DirectorySummary, StorageError> {
        scan_directory_summary(&self.absolute_path)
    }

    /// Load on-demand directory metadata (no recursive walk until summary/size).
    pub fn metadata(&self) -> Result<DirectoryMetadata, StorageError> {
        DirectoryMetadata::load(&self.absolute_path)
    }

    /// Read settable attributes for this directory.
    pub fn attributes(&self) -> Result<StorageAttributes, StorageError> {
        StorageAttributes::from_path(&self.absolute_path)
    }

    /// Apply settable attributes to this directory.
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

    /// Create this directory if it does not already exist.
    pub fn create(&self) -> Result<Self, StorageError> {
        let path = create_directory(&self.absolute_path)?;
        Ok(Self {
            absolute_path: path,
            relative_path: self.relative_path.clone(),
        })
    }

    /// Create this directory and any missing parents.
    pub fn create_all(&self) -> Result<Self, StorageError> {
        let path = create_directory_all(&self.absolute_path)?;
        Ok(Self {
            absolute_path: path,
            relative_path: self.relative_path.clone(),
        })
    }

    /// Enumerate child files in this directory.
    pub fn files(&self) -> Result<Vec<FileStorage>, StorageError> {
        self.files_matching("*", SearchScope::TopDirectoryOnly)
    }

    /// Enumerate child files matching a glob pattern.
    pub fn files_matching(
        &self,
        pattern: &str,
        scope: SearchScope,
    ) -> Result<Vec<FileStorage>, StorageError> {
        collect_matching_paths(&self.absolute_path, pattern, scope, EntryFilter::Files)?
            .into_iter()
            .map(FileStorage::from_existing)
            .collect()
    }

    /// Enumerate child directories in this directory.
    pub fn directories(&self) -> Result<Vec<DirectoryStorage>, StorageError> {
        self.directories_matching("*", SearchScope::TopDirectoryOnly)
    }

    /// Enumerate child directories matching a glob pattern.
    pub fn directories_matching(
        &self,
        pattern: &str,
        scope: SearchScope,
    ) -> Result<Vec<DirectoryStorage>, StorageError> {
        collect_matching_paths(
            &self.absolute_path,
            pattern,
            scope,
            EntryFilter::Directories,
        )?
        .into_iter()
        .map(DirectoryStorage::from_existing)
        .collect()
    }

    /// Enumerate files and directories together.
    pub fn entries(&self) -> Result<Vec<StorageEntry>, StorageError> {
        self.entries_matching("*", SearchScope::TopDirectoryOnly)
    }

    /// Enumerate files and directories together using a glob pattern.
    pub fn entries_matching(
        &self,
        pattern: &str,
        scope: SearchScope,
    ) -> Result<Vec<StorageEntry>, StorageError> {
        collect_matching_paths(&self.absolute_path, pattern, scope, EntryFilter::All)?
            .into_iter()
            .map(StorageEntry::from_existing)
            .collect()
    }

    /// Resolve a file relative to this directory.
    pub fn get_file(&self, relative_path: impl AsRef<Path>) -> Result<FileStorage, StorageError> {
        let path = resolve_relative_child(&self.absolute_path, relative_path.as_ref())?;
        FileStorage::from_existing(path)
    }

    /// Resolve a child directory relative to this directory.
    pub fn get_directory(
        &self,
        relative_path: impl AsRef<Path>,
    ) -> Result<DirectoryStorage, StorageError> {
        let path = resolve_relative_child(&self.absolute_path, relative_path.as_ref())?;
        DirectoryStorage::from_existing(path)
    }

    /// Start a debounced watcher for this directory.
    pub fn watch(&self, config: StorageWatchConfig) -> Result<DirectoryWatchHandle, StorageError> {
        DirectoryWatchHandle::watch(&self.absolute_path, config)
    }

    /// Copy the directory tree to an exact destination path.
    pub fn copy_to(&self, destination: impl AsRef<Path>) -> Result<Self, StorageError> {
        let destination = destination.as_ref();
        let path = copy_directory(&self.absolute_path, destination)?;
        Ok(Self::from_destination(destination, path))
    }

    /// Copy the directory tree with overwrite and progress control.
    pub fn copy_to_with_options(
        &self,
        destination: impl AsRef<Path>,
        options: TransferOptions,
    ) -> Result<Self, StorageError> {
        let destination = destination.as_ref();
        let path = copy_directory_with_options(&self.absolute_path, destination, options)?;
        Ok(Self::from_destination(destination, path))
    }

    /// Move the directory tree to an exact destination path.
    pub fn move_to(&self, destination: impl AsRef<Path>) -> Result<Self, StorageError> {
        let destination = destination.as_ref();
        let path = move_directory(&self.absolute_path, destination)?;
        Ok(Self::from_destination(destination, path))
    }

    /// Move the directory tree with overwrite and progress control.
    pub fn move_to_with_options(
        &self,
        destination: impl AsRef<Path>,
        options: TransferOptions,
    ) -> Result<Self, StorageError> {
        let destination = destination.as_ref();
        let path = move_directory_with_options(&self.absolute_path, destination, options)?;
        Ok(Self::from_destination(destination, path))
    }

    /// Rename the directory inside its current parent directory.
    pub fn rename(&self, new_name: &str) -> Result<Self, StorageError> {
        let path = rename_directory(&self.absolute_path, new_name)?;
        Ok(Self::from_absolute(path))
    }

    /// Delete the directory recursively.
    pub fn delete(&self) -> Result<(), StorageError> {
        delete_directory(&self.absolute_path)
    }

    /// Delete the directory with explicit recursive control.
    pub fn delete_with_options(&self, options: DirectoryDeleteOptions) -> Result<(), StorageError> {
        delete_directory_with_options(&self.absolute_path, options)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EntryFilter {
    Files,
    Directories,
    All,
}

fn collect_matching_paths(
    root: &Path,
    pattern: &str,
    scope: SearchScope,
    filter: EntryFilter,
) -> Result<Vec<PathBuf>, StorageError> {
    let glob = globset::Glob::new(pattern)
        .map_err(|_| StorageError::path_conflict(root.to_path_buf(), "invalid search pattern"))?;
    let matcher = glob.compile_matcher();
    let mut results = Vec::new();
    collect_matching_paths_recursive(root, root, &matcher, scope, filter, &mut results)?;
    results.sort();
    Ok(results)
}

fn collect_matching_paths_recursive(
    root: &Path,
    current: &Path,
    matcher: &globset::GlobMatcher,
    scope: SearchScope,
    filter: EntryFilter,
    results: &mut Vec<PathBuf>,
) -> Result<(), StorageError> {
    for entry in
        fs::read_dir(current).map_err(|err| StorageError::io("read directory", current, err))?
    {
        let entry = entry
            .map_err(|err| StorageError::io("enumerate directory entries for", current, err))?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|err| StorageError::io("read file type for", &path, err))?;
        let relative = path.strip_prefix(root).unwrap_or(&path);
        let matches = matcher.is_match(relative)
            || relative
                .file_name()
                .is_some_and(|name| matcher.is_match(Path::new(name)));

        if matches && include_path(file_type.is_file(), file_type.is_dir(), filter) {
            results.push(path.clone());
        }

        if file_type.is_dir() && scope == SearchScope::AllDirectories {
            collect_matching_paths_recursive(root, &path, matcher, scope, filter, results)?;
        }
    }

    Ok(())
}

fn include_path(is_file: bool, is_dir: bool, filter: EntryFilter) -> bool {
    match filter {
        EntryFilter::Files => is_file,
        EntryFilter::Directories => is_dir,
        EntryFilter::All => is_file || is_dir,
    }
}

fn resolve_relative_child(root: &Path, relative_path: &Path) -> Result<PathBuf, StorageError> {
    if relative_path.is_absolute() {
        return Err(StorageError::path_conflict(
            root.to_path_buf(),
            "relative path must not be absolute",
        ));
    }

    if relative_path.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::Prefix(_) | Component::RootDir
        )
    }) {
        return Err(StorageError::path_conflict(
            root.to_path_buf(),
            "relative path must stay within the directory root",
        ));
    }

    Ok(root.join(relative_path))
}

#[cfg(feature = "async-tokio")]
impl DirectoryStorage {
    /// Async variant of [`Self::create`].
    pub async fn create_async(&self) -> Result<Self, StorageError> {
        let path = crate::operations::create_directory_async(&self.absolute_path).await?;
        Ok(Self {
            absolute_path: path,
            relative_path: self.relative_path.clone(),
        })
    }

    /// Async variant of [`Self::create_all`].
    pub async fn create_all_async(&self) -> Result<Self, StorageError> {
        let path = crate::operations::create_directory_all_async(&self.absolute_path).await?;
        Ok(Self {
            absolute_path: path,
            relative_path: self.relative_path.clone(),
        })
    }

    /// Async variant of [`Self::copy_to_with_options`].
    pub async fn copy_to_async(
        &self,
        destination: impl AsRef<Path>,
        options: TransferOptions,
    ) -> Result<Self, StorageError> {
        let destination = destination.as_ref();
        let path =
            crate::operations::copy_directory_async(&self.absolute_path, destination, options)
                .await?;
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
            crate::operations::move_directory_async(&self.absolute_path, destination, options)
                .await?;
        Ok(Self::from_destination(destination, path))
    }

    /// Async variant of [`Self::rename`].
    pub async fn rename_async(&self, new_name: impl Into<String>) -> Result<Self, StorageError> {
        let path = crate::operations::rename_directory_async(&self.absolute_path, new_name).await?;
        Ok(Self::from_absolute(path))
    }

    /// Async variant of [`Self::delete_with_options`].
    pub async fn delete_async(&self, options: DirectoryDeleteOptions) -> Result<(), StorageError> {
        crate::operations::delete_directory_async(&self.absolute_path, options).await
    }
}
