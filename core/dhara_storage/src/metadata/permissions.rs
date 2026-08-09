//! Effective process capabilities for a path (not a full ACL editor).

use std::fs;
use std::path::Path;

use crate::error::StorageError;
use crate::metadata::attributes::{StorageAttributes, attributes_from_fs_metadata};

pub use dhara_storage_core::StoragePermissions;

/// Probe effective capabilities for `path`.
pub(crate) fn permissions_from_path(path: &Path) -> Result<StoragePermissions, StorageError> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|err| StorageError::io("read metadata for", path, err))?;
    let attrs = attributes_from_fs_metadata(&metadata, path);
    Ok(permissions_from_metadata_and_attrs(path, &metadata, attrs))
}

pub(crate) fn permissions_from_metadata_and_attrs(
    path: &Path,
    metadata: &fs::Metadata,
    attrs: StorageAttributes,
) -> StoragePermissions {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = metadata.permissions().mode();
        // Owner bits as a coarse stand-in; real access may differ via groups/ACLs.
        let can_read = mode & 0o400 != 0;
        let can_write = mode & 0o200 != 0 && !attrs.read_only;
        let can_execute = mode & 0o100 != 0;
        let _ = path;
        StoragePermissions {
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
        StoragePermissions {
            can_read,
            can_write,
            can_modify: can_write,
            can_execute,
        }
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = (path, metadata);
        StoragePermissions {
            can_read: true,
            can_write: !attrs.read_only,
            can_modify: !attrs.read_only,
            can_execute: false,
        }
    }
}
