//! Shared [`StorageMetadata`] trait and load helpers for timestamps / links.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::error::StorageError;

use super::attributes::{StorageAttributes, attributes_from_fs_metadata};
use super::permissions::{StoragePermissions, permissions_from_metadata_and_attrs};
use super::shell_icon::ShellIcon;
use super::temporary::is_temporary_from_metadata;

/// Common metadata contract for files and directories.
///
/// Paths and size live on storage handles, not here. File type labels come from
/// content analysis when available; see [`crate::metadata::StorageType`].
pub trait StorageMetadata {
    /// Last path segment (file or directory name).
    fn name(&self) -> &str;
    /// Filesystem creation timestamp when the platform exposes it.
    fn created_at(&self) -> Option<SystemTime>;
    /// Filesystem last-modified timestamp when the platform exposes it.
    fn modified_at(&self) -> Option<SystemTime>;
    /// Filesystem last-accessed timestamp when the platform exposes it.
    fn accessed_at(&self) -> Option<SystemTime>;
    /// Settable attributes snapshot.
    fn attributes(&self) -> StorageAttributes;
    /// Effective permissions for the current process.
    fn permissions(&self) -> StoragePermissions;
    /// Whether the entry is a symbolic link.
    fn is_symbolic_link(&self) -> bool;
    /// Stored symlink target when the entry is a link and the target was read.
    fn link_target(&self) -> Option<&Path>;
    /// Whether the entry is temporary by attribute and/or temp-directory location.
    fn is_temporary(&self) -> bool;
    /// Lazily loaded shell icon at the default size, when available.
    fn icon(&self) -> Option<&ShellIcon>;
    /// Load a shell icon at an explicit pixel size without caching on `self`.
    fn load_icon_at(&self, size: u32) -> Option<ShellIcon>;
}

/// Shared fields loaded from the filesystem for metadata snapshots.
#[derive(Debug, Clone)]
pub(crate) struct CommonFields {
    pub name: String,
    pub attributes: StorageAttributes,
    pub permissions: StoragePermissions,
    pub is_symbolic_link: bool,
    pub link_target: Option<PathBuf>,
    pub is_temporary: bool,
    pub created_at: Option<SystemTime>,
    pub modified_at: Option<SystemTime>,
    pub accessed_at: Option<SystemTime>,
}

impl CommonFields {
    pub(crate) fn load(absolute_path: &Path) -> Result<(Self, fs::Metadata), StorageError> {
        let metadata = fs::symlink_metadata(absolute_path)
            .map_err(|err| StorageError::io("read metadata for", absolute_path, err))?;

        let name = absolute_path
            .file_name()
            .map(|value| value.to_string_lossy().into_owned())
            .unwrap_or_else(|| absolute_path.as_os_str().to_string_lossy().into_owned());

        let is_symbolic_link = metadata.file_type().is_symlink();
        let link_target = if is_symbolic_link {
            fs::read_link(absolute_path).ok()
        } else {
            None
        };

        let attributes = attributes_from_fs_metadata(&metadata, absolute_path);
        let permissions = permissions_from_metadata_and_attrs(absolute_path, &metadata, attributes);
        let is_temporary = is_temporary_from_metadata(absolute_path, &metadata);

        Ok((
            Self {
                name,
                attributes,
                permissions,
                is_symbolic_link,
                link_target,
                is_temporary,
                created_at: metadata.created().ok(),
                modified_at: metadata.modified().ok(),
                accessed_at: metadata.accessed().ok(),
            },
            metadata,
        ))
    }
}
