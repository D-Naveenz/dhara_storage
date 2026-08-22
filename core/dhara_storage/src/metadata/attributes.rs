//! Settable filesystem attributes — path I/O over core [`StorageAttributes`].

use std::fs;
use std::path::Path;

use crate::error::StorageError;

pub use dhara_storage_core::StorageAttributes;

/// Read settable attributes for an existing path.
pub(crate) fn attributes_from_path(path: &Path) -> Result<StorageAttributes, StorageError> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|err| StorageError::io("read metadata for", path, err))?;
    Ok(attributes_from_fs_metadata(&metadata, path))
}

pub(crate) fn attributes_from_fs_metadata(
    metadata: &fs::Metadata,
    path: &Path,
) -> StorageAttributes {
    StorageAttributes {
        read_only: metadata.permissions().readonly(),
        hidden: is_hidden(metadata, path),
        system: is_system(metadata),
    }
}

/// Apply settable attributes to an existing path (best-effort per platform).
pub fn apply_storage_attributes(path: &Path, attrs: StorageAttributes) -> Result<(), StorageError> {
    #[cfg(windows)]
    {
        apply_attributes_windows(path, attrs)
    }
    #[cfg(not(windows))]
    {
        apply_attributes_unix(path, attrs)
    }
}

#[cfg(windows)]
fn apply_attributes_windows(path: &Path, attrs: StorageAttributes) -> Result<(), StorageError> {
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::Storage::FileSystem::{
        FILE_ATTRIBUTE_HIDDEN, FILE_ATTRIBUTE_NORMAL, FILE_ATTRIBUTE_READONLY,
        FILE_ATTRIBUTE_SYSTEM, FILE_FLAGS_AND_ATTRIBUTES, SetFileAttributesW,
    };
    use windows::core::PCWSTR;

    let mut flags = 0u32;
    if attrs.read_only {
        flags |= FILE_ATTRIBUTE_READONLY.0;
    }
    if attrs.hidden {
        flags |= FILE_ATTRIBUTE_HIDDEN.0;
    }
    if attrs.system {
        flags |= FILE_ATTRIBUTE_SYSTEM.0;
    }
    if flags == 0 {
        flags = FILE_ATTRIBUTE_NORMAL.0;
    }

    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let ok = unsafe { SetFileAttributesW(PCWSTR(wide.as_ptr()), FILE_FLAGS_AND_ATTRIBUTES(flags)) };
    if ok.is_err() {
        return Err(StorageError::io(
            "set file attributes for",
            path,
            std::io::Error::last_os_error(),
        ));
    }
    Ok(())
}

#[cfg(not(windows))]
fn apply_attributes_unix(path: &Path, attrs: StorageAttributes) -> Result<(), StorageError> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|err| StorageError::io("read metadata for", path, err))?;
    let mut permissions = metadata.permissions();
    permissions.set_readonly(attrs.read_only);
    fs::set_permissions(path, permissions)
        .map_err(|err| StorageError::io("set permissions for", path, err))?;
    // Hidden/system are not portable Unix file-mode bits; documented no-ops.
    let _ = (attrs.hidden, attrs.system);
    Ok(())
}

#[cfg(windows)]
fn is_hidden(metadata: &fs::Metadata, _path: &Path) -> bool {
    use std::os::windows::fs::MetadataExt;
    metadata.file_attributes() & 0x2 != 0
}

#[cfg(not(windows))]
fn is_hidden(_metadata: &fs::Metadata, path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with('.'))
}

#[cfg(windows)]
fn is_system(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    metadata.file_attributes() & 0x4 != 0
}

#[cfg(not(windows))]
fn is_system(_metadata: &fs::Metadata) -> bool {
    false
}
