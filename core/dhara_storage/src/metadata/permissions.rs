//! Effective process capabilities for a path (not a full ACL editor).

use std::fs;
use std::path::Path;

use crate::error::StorageError;

use super::attributes::StorageAttributes;

/// Best-effort effective permissions for the **current process**.
///
/// This is not a security boundary (TOCTOU applies) and does not enumerate
/// Windows ACL principals the way Explorer's Security tab does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StoragePermissions {
    /// Whether the current process can read the entry.
    pub can_read: bool,
    /// Whether the current process can write the entry.
    pub can_write: bool,
    /// Whether the current process can modify/replace content (write without read-only).
    pub can_modify: bool,
    /// Whether the current process can execute the entry (files) or search (directories).
    pub can_execute: bool,
}

impl StoragePermissions {
    /// Probe effective capabilities for `path`.
    pub(crate) fn from_path(path: &Path) -> Result<Self, StorageError> {
        let metadata = fs::symlink_metadata(path)
            .map_err(|err| StorageError::io("read metadata for", path, err))?;
        let attrs = StorageAttributes::from_fs_metadata(&metadata, path);
        Ok(Self::from_metadata_and_attrs(path, &metadata, attrs))
    }

    pub(crate) fn from_metadata_and_attrs(
        path: &Path,
        metadata: &fs::Metadata,
        attrs: StorageAttributes,
    ) -> Self {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = metadata.permissions().mode();
            // Owner bits as a coarse stand-in; real access may differ via groups/ACLs.
            let can_read = mode & 0o400 != 0;
            let can_write = mode & 0o200 != 0 && !attrs.read_only;
            let can_execute = mode & 0o100 != 0;
            let _ = path;
            Self {
                can_read,
                can_write,
                can_modify: can_write,
                can_execute,
            }
        }
        #[cfg(windows)]
        {
            // Coarse probe: existence + readonly attribute. Full ACL evaluation is out of scope.
            let can_read = true;
            let can_write = !attrs.read_only;
            let can_execute = metadata.is_file() || metadata.is_dir();
            let _ = path;
            Self {
                can_read,
                can_write,
                can_modify: can_write,
                can_execute,
            }
        }
        #[cfg(not(any(unix, windows)))]
        {
            let _ = (path, metadata);
            Self {
                can_read: true,
                can_write: !attrs.read_only,
                can_modify: !attrs.read_only,
                can_execute: false,
            }
        }
    }
}
